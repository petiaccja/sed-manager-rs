use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SequenceNumber(pub u32);

impl SequenceNumber {
    pub fn initial() -> Self {
        Self(1)
    }

    pub fn fetch_add(&mut self) -> Self {
        let current = *self;
        *self = match self.0.wrapping_add(1) {
            0 => Self(1),
            x => Self(x),
        };
        current
    }
}

impl PartialOrd for SequenceNumber {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.0 == other.0 {
            Some(Ordering::Equal)
        } else if self.0.wrapping_sub(other.0) < u32::MAX / 2 {
            Some(Ordering::Greater)
        } else {
            Some(Ordering::Less)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    #[rstest]
    #[case(SequenceNumber(1), SequenceNumber(2), true)]
    #[case(SequenceNumber(2), SequenceNumber(1), false)]
    #[case(SequenceNumber(2), SequenceNumber(2), false)]
    #[case(SequenceNumber(u32::MAX - 2), SequenceNumber(2), true)]
    #[case(SequenceNumber(2), SequenceNumber(u32::MAX - 2), false)]
    fn sequence_number_lt(#[case] lhs: SequenceNumber, #[case] rhs: SequenceNumber, #[case] expected: bool) {
        assert_eq!(lhs < rhs, expected);
    }

    #[rstest]
    #[case(SequenceNumber(1), SequenceNumber(2))]
    #[case(SequenceNumber(u32::MAX), SequenceNumber(1))]
    fn next(#[case] initial: SequenceNumber, #[case] expected: SequenceNumber) {
        let mut running = initial;
        assert_eq!(running.fetch_add(), initial);
        assert_eq!(running, expected);
    }
}
