//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::collections::{HashMap, HashSet};
use std::num::NonZero;
use std::ops::{Deref as _, DerefMut as _};
use std::path::Path;
use std::sync::Mutex;

use sed_device::{Error, Interface, StorageDevice};
use sed_packet::com_id::{ComId, ComIdExt};
use sed_packet::com_id_request::ComIdRequest;
use sed_packet::discovery::Discovery;
use sed_packet::packet::{COM_PACKET_HEADER_LEN, ComPacket, PACKET_HEADER_LEN, SUB_PACKET_HEADER_LEN};
use sed_packet::session_id::SessionId;
use sed_spec::methods::{Limit, MethodStatus, OptionalLimit, Properties};
use sed_spec::objects::{AuthorityRef, SecurityProviderRef};
use sorbit::ser_de::{FromBytes, ToBytes as _};
use tracing::instrument;

use crate::com_session::ComSession;
use crate::internal_error::Expect;
use crate::packet_session::PacketSession;
use crate::tper::{Opal2Tper, Tper};

pub const VIRTUAL_DEVICE_PATH: &str = "/dev/virtual_device";
pub const BASE_COM_ID: ComId = ComId(3072);
pub const NUM_COM_IDS: u16 = 1;
pub const LOGICAL_SECTOR_COUNT: u64 = 8_589_934_592; // 4 TiB

#[derive(Debug)]
pub struct VirtualDevice {
    tper: Mutex<Tper>,
    sessions: Mutex<Sessions>,
    capabilities: Properties,
}

impl VirtualDevice {
    pub fn new(capabilities: Properties, tper: Tper) -> Self {
        let static_com_ids = (BASE_COM_ID.0..BASE_COM_ID.0 + NUM_COM_IDS).map(ComId);
        let com_sessions = static_com_ids
            .clone()
            .map(|com_id| (com_id, ComSession::new(com_id, capabilities.clone())))
            .collect();
        let packet_sessions = static_com_ids
            .map(|com_id| (com_id, PacketSession::new(com_id, ComIdExt(0), capabilities.clone())))
            .collect();

        Self { tper: tper.into(), sessions: Sessions { com_sessions, packet_sessions }.into(), capabilities }
    }

    /// Start a session on the smallest base ComID.
    ///
    /// This method does not check for authentication, it always starts the
    /// session.
    pub fn insert_session(
        &self,
        host_session_number: u32,
        sp: SecurityProviderRef,
        authority: Option<AuthorityRef>,
    ) -> Result<SessionId, MethodStatus> {
        let tper = self.tper.lock().expect("the virtual device panicked in another thread");
        let mut sessions = self.sessions.lock().expect("the virtual device panicked in another thread");
        let Sessions { packet_sessions, .. } = sessions.deref_mut();

        let packet_session = packet_sessions.get_mut(&BASE_COM_ID).expect("base session accidentally deleted");
        packet_session.insert_session(tper.deref(), host_session_number, sp, authority)
    }

    /// Return a list of the currently active sessions on the ComID.
    pub fn sessions(&self, com_id: ComId, com_id_ext: ComIdExt) -> Result<HashSet<SessionId>, Error> {
        let sessions = self.sessions.lock().expect("the virtual device panicked in another thread");
        let Sessions { packet_sessions, .. } = sessions.deref();
        match packet_sessions.get(&com_id) {
            Some(packet_session) if packet_session.com_id_ext() == com_id_ext => Ok(packet_session.sessions()),
            _ => Err(Error::InvalidProtocolOrComID),
        }
    }

    /// Returns the usual discovery message, bypassing IF-RECV.
    ///
    /// Use this in tests to check device state.
    pub fn discover(&self) -> Discovery {
        self.tper.lock().unwrap().discover(&self.capabilities)
    }

    /// Get information about the device's internal state, like the content of
    /// the tables.
    ///
    /// This function exposes everything without access control checks, and is
    /// helpful to create checks in tests.
    pub fn inspect<Output>(&self, f: impl FnOnce(&Tper) -> Output) -> Output {
        let tper = self.tper.lock().expect("the virtual device panicked in another thread");
        f(&tper)
    }
}

#[async_trait::async_trait]
impl StorageDevice for VirtualDevice {
    fn path(&self) -> Option<&Path> {
        Some(VIRTUAL_DEVICE_PATH.as_ref())
    }

    fn interface(&self) -> Interface {
        Interface::Other
    }

    fn model_number(&self) -> String {
        "Virtual Device".into()
    }

    fn serial_number(&self) -> String {
        "SERIAL01".into()
    }

    fn firmware_revision(&self) -> String {
        "FWREV01".into()
    }

    fn is_security_supported(&self) -> bool {
        true
    }

    fn is_removable(&self) -> bool {
        true
    }

    #[instrument(skip(self), ret, err)]
    async fn logical_sector_size(&self) -> Result<u32, Error> {
        Ok(512)
    }

    #[instrument(skip(self), ret, err)]
    async fn logical_sector_count(&self) -> Result<u64, Error> {
        Ok(LOGICAL_SECTOR_COUNT)
    }

    #[instrument(skip(self, data), fields(len = debug(data.len())), ret, err)]
    async fn security_send(&self, security_protocol: u8, protocol_specific: [u8; 2], data: &[u8]) -> Result<(), Error> {
        if data.len() > self.capabilities.max_gross_compacket_size.get() {
            return Err(Error::BufferTooLarge);
        }

        let mut tper = self.tper.lock().expect("the virtual device panicked in another thread");
        let mut sessions = self.sessions.lock().expect("the virtual device panicked in another thread");
        let Sessions { com_sessions, packet_sessions } = sessions.deref_mut();

        let com_id = u16::from_be_bytes(protocol_specific);
        match (security_protocol, com_id) {
            // Discovery: ignore
            (0x01, 0x0001) => Ok(()),
            // Communication layer
            (0x02, com_id) if let Some(session) = com_sessions.get_mut(&ComId(com_id)) => {
                match ComIdRequest::from_bytes(data) {
                    Ok(request) => {
                        let _ = session.push(packet_sessions, request);
                        Ok(())
                    }
                    Err(_) => Err(Error::InvalidArgument),
                }
            }
            // Packet layer
            (0x01, com_id) if let Some(session) = packet_sessions.get_mut(&ComId(com_id)) => {
                match ComPacket::from_bytes(data) {
                    Ok(com_packet) => {
                        session.push(&mut tper, com_packet);
                        Ok(())
                    }
                    Err(_) => Err(Error::InvalidArgument),
                }
            }
            (_, _) => Err(Error::InvalidProtocolOrComID),
        }
    }

    #[instrument(skip(self), err)]
    async fn security_recv(
        &self,
        security_protocol: u8,
        protocol_specific: [u8; 2],
        len: usize,
    ) -> Result<Vec<u8>, Error> {
        let tper = self.tper.lock().expect("the virtual device panicked in another thread");
        let mut sessions = self.sessions.lock().expect("the virtual device panicked in another thread");
        let Sessions { com_sessions, packet_sessions } = sessions.deref_mut();

        let com_id = u16::from_be_bytes(protocol_specific);
        match (security_protocol, com_id) {
            // Discovery
            (0x01, 0x0001) => {
                let mut message = tper.pop_discovery(&self.capabilities);
                // In case `len` is too small, we must return the truncated buffer.
                message.resize(len, 0);
                Ok(message)
            }
            // Get ComID
            (0x02, 0x0000) => Err(Error::NotSupported),
            // Communication layer
            (0x02, com_id) if let Some(session) = com_sessions.get_mut(&ComId(com_id)) => {
                let response = session.pop();
                let mut message = response.to_bytes().expect_serialize();
                message.resize(len, 0);
                Ok(message)
            }
            // Packet layer
            (0x01, com_id) if let Some(session) = packet_sessions.get_mut(&ComId(com_id)) => {
                session.pop(len).map(|com_packet| {
                    let mut bytes = com_packet.to_bytes().expect_serialize();
                    bytes.resize(len, 0);
                    bytes
                })
            }
            (_, _) => Err(Error::InvalidProtocolOrComID),
        }
    }
}

impl Default for VirtualDevice {
    /// Create a new virtual device.
    ///
    /// The device's configuration is the preconfiguration for the Opal 2.0 SSC.
    fn default() -> Self {
        let tper = Tper::Opal2(Opal2Tper::default());
        let capabilities = CAPABILITIES;

        Self::new(capabilities, tper)
    }
}

#[derive(Debug, Default)]
struct Sessions {
    com_sessions: HashMap<ComId, ComSession>,
    packet_sessions: HashMap<ComId, PacketSession>,
}

pub struct VirtualDeviceBuilder {
    capabilities: Properties,
}

impl VirtualDeviceBuilder {
    pub fn with_capabilities(self, capabilities: Properties) -> Self {
        Self { capabilities, ..self }
    }

    pub fn opal_2(self) -> VirtualDevice {
        let tper = Tper::Opal2(Opal2Tper::default());
        VirtualDevice::new(self.capabilities, tper)
    }
}

/// Give it a realistic number based on what I've seen on real hardware.
const MAX_GROSS_COM_PACKET_SIZE: usize = 65536;

pub const CAPABILITIES: Properties = Properties {
    max_methods: Limit::Unlimited,
    max_subpackets: Limit::Unlimited,
    max_gross_packet_size: Limit::Limited(NonZero::new(MAX_GROSS_COM_PACKET_SIZE - COM_PACKET_HEADER_LEN).unwrap()),
    max_packets: Limit::Unlimited,
    max_gross_compacket_size: Limit::Limited(NonZero::new(MAX_GROSS_COM_PACKET_SIZE).unwrap()),
    max_gross_compacket_response_size: Limit::Limited(NonZero::new(MAX_GROSS_COM_PACKET_SIZE).unwrap()),
    max_sessions: Some(Limit::Unlimited),
    max_read_sessions: Some(Limit::Unlimited),
    max_ind_token_size: Limit::Limited(
        NonZero::new(MAX_GROSS_COM_PACKET_SIZE - COM_PACKET_HEADER_LEN - PACKET_HEADER_LEN - SUB_PACKET_HEADER_LEN)
            .unwrap(),
    ),
    max_agg_token_size: Limit::Limited(
        NonZero::new(MAX_GROSS_COM_PACKET_SIZE - COM_PACKET_HEADER_LEN - PACKET_HEADER_LEN - SUB_PACKET_HEADER_LEN)
            .unwrap(),
    ),
    max_authentications: Some(Limit::Unlimited),
    max_transaction_limit: Some(Limit::Unlimited),
    def_session_timeout: Some(Limit::Unlimited),
    max_session_timeout: Some(Limit::Unlimited),
    min_session_timeout: Some(OptionalLimit::Unsupported),
    def_trans_timeout: Some(Limit::Unlimited),
    max_trans_timeout: Some(Limit::Unlimited),
    min_trans_timeout: Some(OptionalLimit::Unsupported),
    max_com_id_time: Some(Limit::Unlimited),
    continued_tokens: false,
    seq_numbers: false,
    ack_nak: false,
    asynchronous: false,
};
