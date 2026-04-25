use crate::packet::Packet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionId {
    pub hsn: u32,
    pub tsn: u32,
}

impl SessionId {
    pub const MANAGEMENT: Self = Self { hsn: 0, tsn: 0 };

    pub fn of(packet: &Packet) -> Self {
        Self { hsn: packet.host_session_number, tsn: packet.tper_session_number }
    }

    pub fn assign(&self, packet: Packet) -> Packet {
        Packet { tper_session_number: self.tsn, host_session_number: self.hsn, ..packet }
    }
}

impl core::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Session{{ HSN={}, TSN={} }}", self.hsn, self.tsn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn of() {
        let packet = Packet { tper_session_number: 1, host_session_number: 2, ..Default::default() };
        assert_eq!(SessionId::of(&packet), SessionId { hsn: 2, tsn: 1 })
    }

    #[test]
    fn assign() {
        let session = SessionId { hsn: 1, tsn: 2 };
        let packet = session.assign(Packet::default());
        assert_eq!(packet.host_session_number, 1);
        assert_eq!(packet.tper_session_number, 2);
    }
}
