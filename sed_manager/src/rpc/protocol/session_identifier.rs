//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::messaging::packet::Packet;

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub struct SessionIdentifier {
    pub hsn: u32,
    pub tsn: u32,
}

impl From<&Packet> for SessionIdentifier {
    fn from(value: &Packet) -> Self {
        Self { hsn: value.host_session_number, tsn: value.tper_session_number }
    }
}

pub const CONTROL_SESSION_ID: SessionIdentifier = SessionIdentifier { hsn: 0, tsn: 0 };
