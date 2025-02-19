use super::uid::UID;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UIDRange {
    start: UID,
    end: UID,
    step: u64,
}

impl UIDRange {
    pub const fn new_range(start: UID, end: UID, step: u64) -> Self {
        Self { start, end, step }
    }

    pub const fn new_count(start: UID, count: u64, step: u64) -> Self {
        Self { start, end: UID::new(start.value() + count * step), step }
    }

    pub const fn nth(&self, offset: u64) -> Option<UID> {
        let (start, step) = (self.start.value(), self.step);
        let uid = UID::new(start + offset * step);
        match self.contains(uid) {
            true => Some(uid),
            false => None,
        }
    }

    pub const fn contains(&self, uid: UID) -> bool {
        let (start, end, step) = (self.start.value(), self.end.value(), self.step);
        let value = uid.value();
        start <= value && value < end && (value - start) % step == 0
    }

    pub const fn index_of(&self, uid: UID) -> Option<u64> {
        match self.contains(uid) {
            true => Some((uid.value() - self.start.value()) / self.step),
            false => None,
        }
    }
}

impl From<UID> for UIDRange {
    fn from(value: UID) -> Self {
        Self::new_count(value, 1, 1)
    }
}

#[cfg(test)]
mod tests {
    use super::UIDRange;
    use super::UID;

    const BASE: UID = UID::new(1000);

    #[test]
    fn nth() {
        let range = UIDRange::new_count(BASE, 10, 1);
        assert_eq!(range.nth(0), Some(BASE));
        assert_eq!(range.nth(9), Some(UID::new(BASE.value() + 9)));
        assert_eq!(range.nth(10), None);
    }

    #[test]
    fn nth_stepped() {
        let range = UIDRange::new_count(BASE, 10, 3);
        assert_eq!(range.nth(0), Some(BASE));
        assert_eq!(range.nth(9), Some(UID::new(BASE.value() + 27)));
        assert_eq!(range.nth(10), None);
    }

    #[test]
    fn contains() {
        let range = UIDRange::new_count(BASE, 10, 1);
        assert_eq!(range.contains(UID::new(BASE.value() - 1)), false);
        assert_eq!(range.contains(UID::new(BASE.value() + 0)), true);
        assert_eq!(range.contains(UID::new(BASE.value() + 9)), true);
        assert_eq!(range.contains(UID::new(BASE.value() + 10)), false);
    }

    #[test]
    fn contains_stepped() {
        let range = UIDRange::new_count(BASE, 10, 3);
        assert_eq!(range.contains(UID::new(BASE.value() - 3)), false);
        assert_eq!(range.contains(UID::new(BASE.value() - 1)), false);
        assert_eq!(range.contains(UID::new(BASE.value() + 0)), true);
        assert_eq!(range.contains(UID::new(BASE.value() + 1)), false);
        assert_eq!(range.contains(UID::new(BASE.value() + 3)), true);
        assert_eq!(range.contains(UID::new(BASE.value() + 27)), true);
        assert_eq!(range.contains(UID::new(BASE.value() + 28)), false);
        assert_eq!(range.contains(UID::new(BASE.value() + 30)), false);
    }

    #[test]
    fn index_of() {
        let range = UIDRange::new_count(BASE, 10, 1);
        assert_eq!(range.index_of(UID::new(BASE.value() - 1)), None);
        assert_eq!(range.index_of(UID::new(BASE.value() + 0)), Some(0));
        assert_eq!(range.index_of(UID::new(BASE.value() + 9)), Some(9));
        assert_eq!(range.index_of(UID::new(BASE.value() + 10)), None);
    }

    #[test]
    fn index_of_stepped() {
        let range = UIDRange::new_count(BASE, 10, 3);
        assert_eq!(range.index_of(UID::new(BASE.value() - 3)), None);
        assert_eq!(range.index_of(UID::new(BASE.value() - 1)), None);
        assert_eq!(range.index_of(UID::new(BASE.value() + 0)), Some(0));
        assert_eq!(range.index_of(UID::new(BASE.value() + 1)), None);
        assert_eq!(range.index_of(UID::new(BASE.value() + 3)), Some(1));
        assert_eq!(range.index_of(UID::new(BASE.value() + 27)), Some(9));
        assert_eq!(range.index_of(UID::new(BASE.value() + 28)), None);
        assert_eq!(range.index_of(UID::new(BASE.value() + 30)), None);
    }
}
