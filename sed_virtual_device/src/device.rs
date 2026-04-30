use std::collections::{HashMap, HashSet};
use std::ops::{Deref as _, DerefMut as _};
use std::sync::Mutex;

use sed_device::{Device, Error, Interface};
use sed_packet::com_id::ComIdRequest;
use sed_packet::packet::ComPacket;
use sed_packet::session_id::SessionId;
use sed_spec::objects::{AuthorityRef, SecurityProviderRef};
use sorbit::ser_de::{FromBytes, ToBytes as _};

use crate::com_id::{ComId, ComIdExt};
use crate::com_session::ComSession;
use crate::packet_session::PacketSession;
use crate::tper::{Opal2TPer, TPer};

pub const BASE_COM_ID: ComId = ComId(3072);
pub const NUM_COM_IDS: u16 = 1;

#[derive(Debug)]
pub struct VirtualDevice {
    tper: Mutex<TPer>,
    sessions: Mutex<Sessions>,
}

impl VirtualDevice {
    /// Create a new virtual device.
    ///
    /// The device's configuration is the preconfiguration for the Opal 2.0 SSC.
    pub fn new() -> Self {
        let static_com_ids = (BASE_COM_ID.0..BASE_COM_ID.0 + NUM_COM_IDS).map(|com_id| ComId(com_id));
        let com_sessions = static_com_ids.clone().map(|com_id| (com_id, ComSession::new(com_id))).collect();
        let packet_sessions = static_com_ids.map(|com_id| (com_id, PacketSession::new(com_id, ComIdExt(0)))).collect();

        Self {
            tper: TPer::Opal2(Opal2TPer::default()).into(),
            sessions: Sessions { com_sessions, packet_sessions }.into(),
        }
    }

    /// Start a session on the smallest base ComID.
    ///
    /// This method does not check for authentication, it always starts the
    /// session.
    pub fn start_session(
        &self,
        host_session_number: u32,
        sp: SecurityProviderRef,
        authority: Option<AuthorityRef>,
    ) -> SessionId {
        let mut sessions = self.sessions.lock().expect("the virtual device panicked in another thread");
        let Sessions { packet_sessions, .. } = sessions.deref_mut();

        let packet_session = packet_sessions.get_mut(&BASE_COM_ID).expect("base session accidentally deleted");
        packet_session.start_session(host_session_number, sp, authority)
    }

    /// Return a list of the currently active sessions on the ComID.
    pub fn sessions(&self, com_id: u16, com_id_ext: u16) -> Result<HashSet<SessionId>, Error> {
        let sessions = self.sessions.lock().expect("the virtual device panicked in another thread");
        let Sessions { packet_sessions, .. } = sessions.deref();
        match packet_sessions.get(&ComId(com_id)) {
            Some(packet_session) if packet_session.com_id_ext().0 == com_id_ext => Ok(packet_session.sessions()),
            _ => Err(Error::InvalidProtocolOrComID),
        }
    }
}

impl Device for VirtualDevice {
    fn path(&self) -> Option<String> {
        None
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

    fn security_send(&self, security_protocol: u8, protocol_specific: [u8; 2], data: &[u8]) -> Result<(), Error> {
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
                    Ok(request) => Ok(session.push(packet_sessions, request)),
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

    fn security_recv(&self, security_protocol: u8, protocol_specific: [u8; 2], len: usize) -> Result<Vec<u8>, Error> {
        let mut tper = self.tper.lock().expect("the virtual device panicked in another thread");
        let mut sessions = self.sessions.lock().expect("the virtual device panicked in another thread");
        let Sessions { com_sessions, packet_sessions } = sessions.deref_mut();

        let com_id = u16::from_be_bytes(protocol_specific);
        match (security_protocol, com_id) {
            // Discovery
            (0x01, 0x0001) => {
                let mut message = tper.pop_discovery();
                // In case `len` is too small, we must return the truncated buffer.
                message.resize(len, 0);
                Ok(message)
            }
            // Get ComID
            (0x02, 0x0000) => Err(Error::NotSupported),
            // Communication layer
            (0x02, com_id) if let Some(session) = com_sessions.get_mut(&ComId(com_id)) => {
                let response = session.pop();
                let mut message = response.to_bytes().expect("serializing ComID response failed");
                message.resize(len, 0);
                Ok(message)
            }
            // Packet layer
            (0x01, com_id) if let Some(session) = packet_sessions.get_mut(&ComId(com_id)) => {
                session.pop(len).map(|com_packet| {
                    let mut bytes = com_packet.to_bytes().expect("serializing ComPacket failed");
                    bytes.resize(len, 0);
                    bytes
                })
            }
            (_, _) => Err(Error::InvalidProtocolOrComID),
        }
    }
}

#[derive(Debug, Default)]
struct Sessions {
    com_sessions: HashMap<ComId, ComSession>,
    packet_sessions: HashMap<ComId, PacketSession>,
}
