//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

//! Implements support for identify & security send/receive commands for NVMe devices.
//!
//! Uses native NVMe admin command ioctl's, i.e. no SCSI or other translation required.

use core::ptr::null_mut;
use std::path::Path;

use sorbit::ser_de::FromBytes as _;

use crate::linux::ioctl_device::IoctlDevice;
use crate::shared::nvme::{GenericStatusCode, IdentifyController, Opcode, StatusCode, StatusField};
use crate::{Device, Error, Interface};

pub use ioctl::NvmeIoctlDevice;

pub struct NvmeDevice {
    ioctl_device: IoctlDevice,
    desc: IdentifyController,
}

impl NvmeDevice {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let ioctl_device = IoctlDevice::open(path).await?;
        let desc = ioctl_device.identify_controller().await?;
        Ok(Self { ioctl_device, desc })
    }
}

#[async_trait::async_trait]
impl Device for NvmeDevice {
    fn path(&self) -> Option<&Path> {
        Some(self.ioctl_device.path())
    }

    fn interface(&self) -> Interface {
        Interface::NVMe
    }

    fn model_number(&self) -> String {
        self.desc.model_number_as_str()
    }

    fn serial_number(&self) -> String {
        self.desc.serial_number_as_str()
    }

    fn firmware_revision(&self) -> String {
        self.desc.firmware_revision_as_str()
    }

    fn is_security_supported(&self) -> bool {
        self.desc.security_send_receive_supported
    }

    fn is_removable(&self) -> bool {
        // The open path is the controller (e.g. `/dev/nvme0`), not a namespace block
        // device, so there's no sysfs `removable` file to read (that only exists
        // per-namespace, e.g. `/sys/block/nvme0n1/removable`). NVMe SSDs aren't
        // meaningfully removable in practice anyway.
        false
    }

    async fn security_send(&self, security_protocol: u8, protocol_specific: [u8; 2], data: &[u8]) -> Result<(), Error> {
        if !self.is_security_supported() {
            return Err(Error::SecurityNotSupported);
        }
        let protocol_specific = u16::from_be_bytes(protocol_specific);
        self.ioctl_device.security_send(security_protocol, protocol_specific, data).await
    }

    async fn security_recv(
        &self,
        security_protocol: u8,
        protocol_specific: [u8; 2],
        len: usize,
    ) -> Result<Vec<u8>, Error> {
        if !self.is_security_supported() {
            return Err(Error::SecurityNotSupported);
        }
        let protocol_specific = u16::from_be_bytes(protocol_specific);
        let mut data = vec![0; len];
        self.ioctl_device.security_receive(security_protocol, protocol_specific, &mut data).await?;
        Ok(data)
    }
}

fn make_cdw10(security_protocol: u8, protocol_specific: u16) -> u32 {
    let sps = protocol_specific.to_be_bytes();
    u32::from_be_bytes([security_protocol, sps[0], sps[1], 0])
}

/// Check if the `ioctl` return value indicates an NVMe error.
/// The NVMe status is encoded in the lowest 11 bits of the value returned by `ioctl`.
fn check_ioctl_err(ioctl_err: i32) -> Result<(), Error> {
    let ioctl_err = ioctl_err as u32;
    let shifted = ioctl_err << 17;
    let status = StatusField::from_bytes(&shifted.to_le_bytes())
        .map(|status_field| status_field.status_code())
        .unwrap_or(StatusCode::InvalidStatusField);
    match status {
        StatusCode::Generic(GenericStatusCode::Success) => match ioctl_err {
            0 => Ok(()),
            _ => Err(Error::NVMeError(StatusCode::Unknown(0))),
        },
        _ => Err(Error::NVMeError(status)),
    }
}

#[derive(Debug)]
#[repr(C)]
struct NvmeAdminCommand {
    opcode: Opcode,
    flags: u8,
    rsvd1: u16,
    nsid: u32,
    cdw2: u32,
    cdw3: u32,
    metadata: *mut u8,
    addr: *mut u8,
    metadata_len: u32,
    data_len: u32,
    cdw10: u32,
    cdw11: u32,
    cdw12: u32,
    cdw13: u32,
    cdw14: u32,
    cdw15: u32,
    timeout_ms: u32,
    result: u32,
}

impl Default for NvmeAdminCommand {
    fn default() -> Self {
        Self {
            opcode: Opcode::IdentifyController,
            flags: 0,
            rsvd1: 0,
            nsid: 0,
            cdw2: 0,
            cdw3: 0,
            metadata: null_mut(),
            addr: null_mut(),
            metadata_len: 0,
            data_len: 0,
            cdw10: 0,
            cdw11: 0,
            cdw12: 0,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
            timeout_ms: 0,
            result: 0,
        }
    }
}

mod ioctl {
    use super::*;

    const NVME_ADMIN_CMD_OPCODE: rustix::ioctl::Opcode =
        rustix::ioctl::opcode::read_write::<NvmeAdminCommand>(b'N', 0x41);

    use command_ioctl::NvmeAdminCommandIoctl;

    mod command_ioctl {
        use rustix::ioctl;

        use super::*;

        /// Execute an NVMe admin command via IOCTLs using [`rustix`].
        ///
        /// The structure MUST be `#[repr(C)]` because `self` is passed directly to the
        /// ICOTL as a `void*`. In order to do this, [`Self::command`] must be at the offset
        /// of zero in the struct. [`Self::buffer`] is never touched be the IOCTL, and must
        /// be the second field.
        #[repr(C)]
        pub struct NvmeAdminCommandIoctl {
            /// The [`NvmeAdminCommand::addr`] and [`NvmeAdminCommand::data_len`]
            /// fields point to [`Self::buffer`].
            command: NvmeAdminCommand,
            /// Set to `None` once the value is extracted and returned by [`ioctl::Ioctl::output_from_ptr`].
            buffer: Option<Vec<u8>>,
        }

        impl NvmeAdminCommandIoctl {
            /// Creates an IOCTL by setting the [`NvmeAdminCommand::addr`] and [`NvmeAdminCommand::data_len`]
            /// fields to point to `buffer`.
            pub fn new(mut command: NvmeAdminCommand, mut buffer: Vec<u8>) -> Self {
                command.addr = buffer.as_mut_ptr();
                command.data_len = buffer.len() as u32;
                Self { command, buffer: Some(buffer) }
            }
        }

        // SAFETY: `command.addr` is a mutable pointer, so cloning `self` and
        // sending it to another thread is a bit of data race when writing `command.addr`.
        // Since `command.addr` always points to `buffer`, and both fields are private,
        // users cannot write the pointer anyway.
        unsafe impl Send for NvmeAdminCommandIoctl {}

        unsafe impl ioctl::Ioctl for NvmeAdminCommandIoctl {
            type Output = (i32, Vec<u8>);

            const IS_MUTATING: bool = true;

            fn opcode(&self) -> ioctl::Opcode {
                NVME_ADMIN_CMD_OPCODE
            }

            fn as_ptr(&mut self) -> *mut core::ffi::c_void {
                // `command` is the first field of a `#[repr(C)]` struct, so this is equivalent to
                // pointing at `command` alone as far as the ioctl (encoded for `NvmeAdminCommand`'s
                // size) is concerned, while still giving `output_from_ptr` a way back to `buffer`.
                (self as *mut Self).cast()
            }

            unsafe fn output_from_ptr(
                out: ioctl::IoctlOutput,
                extract_output: *mut core::ffi::c_void,
            ) -> rustix::io::Result<Self::Output> {
                // Remember that `as_ptr` returnes a `*mut Self`.
                let this = extract_output.cast::<Self>();
                let buffer = unsafe { (*this).buffer.take() }.expect("output_from_ptr is called at most once");
                Ok((out, buffer))
            }
        }
    }

    pub trait NvmeIoctlDevice {
        async fn identify_controller(&self) -> Result<IdentifyController, Error>;

        async fn security_send(
            &self,
            security_protocol: u8,
            security_protocol_specific: u16,
            data_out: &[u8],
        ) -> Result<(), Error>;

        async fn security_receive(
            &self,
            security_protocol: u8,
            security_protocol_specific: u16,
            data_in: &mut [u8],
        ) -> Result<(), Error>;
    }

    impl NvmeIoctlDevice for IoctlDevice {
        async fn identify_controller(&self) -> Result<IdentifyController, Error> {
            let buffer = vec![0_u8; 4096];
            let command =
                NvmeAdminCommand { opcode: Opcode::IdentifyController, cdw10: 0x0000_0001, ..Default::default() };
            let (ioctl_err, buffer) = self.ioctl(NvmeAdminCommandIoctl::new(command, buffer)).await?;
            check_ioctl_err(ioctl_err)?;
            IdentifyController::from_bytes(&buffer).map_err(|_| Error::InterfaceNotSupported)
        }

        async fn security_send(
            &self,
            security_protocol: u8,
            security_protocol_specific: u16,
            data_out: &[u8],
        ) -> Result<(), Error> {
            let cdw11 = data_out.len() as u32; // Data length duplicated.
            // Copied into an owned buffer (rather than pointing `addr` at `data_out` directly)
            // so the whole `NvmeAdminCommandIoctl` genuinely owns everything `addr` points to,
            // instead of relying on the caller never dropping this future early.
            let buffer = data_out.to_vec();
            let command = NvmeAdminCommand {
                opcode: Opcode::SecuritySend,
                cdw10: make_cdw10(security_protocol, security_protocol_specific),
                cdw11,
                ..Default::default()
            };
            let (ioctl_err, _buffer) = self.ioctl(NvmeAdminCommandIoctl::new(command, buffer)).await?;
            check_ioctl_err(ioctl_err)
        }

        async fn security_receive(
            &self,
            security_protocol: u8,
            security_protocol_specific: u16,
            data_in: &mut [u8],
        ) -> Result<(), Error> {
            let cdw11 = data_in.len() as u32; // Data length duplicated.
            let buffer = vec![0_u8; data_in.len()];
            let command = NvmeAdminCommand {
                opcode: Opcode::SecurityReceive,
                cdw10: make_cdw10(security_protocol, security_protocol_specific),
                cdw11,
                ..Default::default()
            };
            let (ioctl_err, buffer) = self.ioctl(NvmeAdminCommandIoctl::new(command, buffer)).await?;
            check_ioctl_err(ioctl_err)?;
            data_in.copy_from_slice(&buffer);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_ioctl_err_success() {
        let ioctl_err = 0b000_00000000;
        assert_eq!(check_ioctl_err(ioctl_err), Ok(()));
    }

    #[test]
    fn check_ioctl_err_nvme_err() {
        let ioctl_err = 0b1_000_00000001;
        assert_eq!(
            check_ioctl_err(ioctl_err),
            Err(Error::NVMeError(StatusCode::Generic(GenericStatusCode::InvalidCommandOpcode)))
        );
    }

    #[test]
    fn check_ioctl_err_non_nvme_err() {
        let ioctl_err = 0b1_000_00000000;
        assert_eq!(check_ioctl_err(ioctl_err), Err(Error::NVMeError(StatusCode::Unknown(0))));
    }
}
