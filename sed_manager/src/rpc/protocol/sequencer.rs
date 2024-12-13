use crate::messaging::packet::AckType;

#[derive(PartialEq, Eq, Debug)]
pub enum AckAction {
    ACK,
    NAK,
    Resend,
    Ignore,
    Pass,
}

pub struct Sequencer {
    expected: u32,
    acknowledged: u32,
    not_acknowledged: (u32, bool),
}

impl Sequencer {
    pub fn new() -> Self {
        Self { expected: 1, acknowledged: 0, not_acknowledged: (0, false) }
    }

    pub fn update(&mut self, sequence_number: u32) -> AckAction {
        if sequence_number == self.expected {
            self.expected += 1;
            AckAction::ACK
        } else if sequence_number > self.expected && self.not_acknowledged.0 != self.expected {
            self.not_acknowledged = (self.expected, true);
            AckAction::NAK
        } else if sequence_number == self.expected - 1 {
            AckAction::Resend
        } else {
            AckAction::Ignore
        }
    }

    pub fn take(&mut self) -> Option<(AckType, u32)> {
        if self.not_acknowledged.1 {
            self.not_acknowledged.1 = false;
            self.acknowledged = self.not_acknowledged.0 - 1;
            Some((AckType::NAK, self.not_acknowledged.0))
        } else if self.acknowledged < self.expected - 1 {
            self.acknowledged = self.expected - 1;
            Some((AckType::ACK, self.acknowledged))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_sequential() {
        let mut state = Sequencer::new();
        assert_eq!(state.update(1), AckAction::ACK);
        assert_eq!(state.update(2), AckAction::ACK);
    }

    #[test]
    fn update_duplicate_old() {
        let mut state = Sequencer::new();
        assert_eq!(state.update(1), AckAction::ACK);
        assert_eq!(state.update(2), AckAction::ACK);
        assert_eq!(state.update(1), AckAction::Ignore);
    }

    #[test]
    fn update_duplicate_current() {
        let mut state = Sequencer::new();
        assert_eq!(state.update(1), AckAction::ACK);
        assert_eq!(state.update(2), AckAction::ACK);
        assert_eq!(state.update(2), AckAction::Resend);
    }

    #[test]
    fn update_missing() {
        let mut state = Sequencer::new();
        assert_eq!(state.update(1), AckAction::ACK);
        assert_eq!(state.update(3), AckAction::NAK);
    }

    #[test]
    fn take_sequential() {
        let mut state = Sequencer::new();
        state.update(1);
        state.update(2);
        assert_eq!(state.take(), Some((AckType::ACK, 2)));
        assert_eq!(state.take(), None);
    }

    #[test]
    fn take_duplicate_old() {
        let mut state = Sequencer::new();
        state.update(1);
        state.update(2);
        assert_eq!(state.take(), Some((AckType::ACK, 2)));
        state.update(1);
        assert_eq!(state.take(), None);
    }

    #[test]
    fn take_duplicate_current() {
        let mut state = Sequencer::new();
        state.update(1);
        assert_eq!(state.take(), Some((AckType::ACK, 1)));
        state.update(2);
        state.update(2);
        assert_eq!(state.take(), Some((AckType::ACK, 2)));
        assert_eq!(state.take(), None);
    }

    #[test]
    fn take_missing() {
        let mut state = Sequencer::new();
        state.update(1);
        state.update(3);
        assert_eq!(state.take(), Some((AckType::NAK, 2)));
        assert_eq!(state.take(), None);
    }
}
