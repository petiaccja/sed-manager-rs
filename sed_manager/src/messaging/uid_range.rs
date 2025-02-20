use super::uid::{ObjectUID, UID};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UIDRange {
    start: UID,
    end: UID,
    step: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectUIDRange<const TABLE: u64>(UIDRange);

impl UIDRange {
    pub const fn new_range(start: UID, end: UID, step: u64) -> Self {
        Self { start, end, step }
    }

    pub const fn new_count(start: UID, count: u64, step: u64) -> Self {
        Self { start, end: UID::new(start.as_u64() + count * step), step }
    }

    pub const fn nth(&self, offset: u64) -> Option<UID> {
        let (start, step) = (self.start.as_u64(), self.step);
        let uid = UID::new(start + offset * step);
        match self.contains(uid) {
            true => Some(uid),
            false => None,
        }
    }

    pub const fn contains(&self, uid: UID) -> bool {
        let (start, end, step) = (self.start.as_u64(), self.end.as_u64(), self.step);
        let value = uid.as_u64();
        start <= value && value < end && (value - start) % step == 0
    }

    pub const fn index_of(&self, uid: UID) -> Option<u64> {
        match self.contains(uid) {
            true => Some((uid.as_u64() - self.start.as_u64()) / self.step),
            false => None,
        }
    }
}

impl<const TABLE: u64> ObjectUIDRange<TABLE> {
    pub const fn new_range(start: ObjectUID<TABLE>, end: ObjectUID<TABLE>, step: u64) -> Self {
        Self(UIDRange::new_range(start.as_uid(), end.as_uid(), step))
    }

    pub const fn new_count(start: ObjectUID<TABLE>, count: u64, step: u64) -> Self {
        Self(UIDRange::new_count(start.as_uid(), count, step))
    }

    pub const fn nth(&self, offset: u64) -> Option<ObjectUID<TABLE>> {
        if let Some(uid) = self.0.nth(offset) {
            Some(ObjectUID::<TABLE>::new(uid.as_u64()))
        } else {
            None
        }
    }

    pub const fn contains(&self, uid: ObjectUID<TABLE>) -> bool {
        self.0.contains(uid.as_uid())
    }

    pub const fn index_of(&self, uid: ObjectUID<TABLE>) -> Option<u64> {
        self.0.index_of(uid.as_uid())
    }

    pub const fn as_uid_range(&self) -> UIDRange {
        self.0
    }
}

impl From<UID> for UIDRange {
    fn from(value: UID) -> Self {
        Self::new_count(value, 1, 1)
    }
}

impl<const TABLE: u64> From<ObjectUID<TABLE>> for ObjectUIDRange<TABLE> {
    fn from(value: ObjectUID<TABLE>) -> Self {
        Self::new_count(value, 1, 1)
    }
}

impl<const TABLE: u64> TryFrom<UIDRange> for ObjectUIDRange<TABLE> {
    type Error = UIDRange;
    fn try_from(value: UIDRange) -> Result<Self, Self::Error> {
        if let (Ok(_), Ok(_)) = (ObjectUID::<TABLE>::try_from(value.start), ObjectUID::<TABLE>::try_from(value.end)) {
            Ok(Self(value))
        } else {
            Err(value)
        }
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
        assert_eq!(range.nth(9), Some(UID::new(BASE.as_u64() + 9)));
        assert_eq!(range.nth(10), None);
    }

    #[test]
    fn nth_stepped() {
        let range = UIDRange::new_count(BASE, 10, 3);
        assert_eq!(range.nth(0), Some(BASE));
        assert_eq!(range.nth(9), Some(UID::new(BASE.as_u64() + 27)));
        assert_eq!(range.nth(10), None);
    }

    #[test]
    fn contains() {
        let range = UIDRange::new_count(BASE, 10, 1);
        assert_eq!(range.contains(UID::new(BASE.as_u64() - 1)), false);
        assert_eq!(range.contains(UID::new(BASE.as_u64() + 0)), true);
        assert_eq!(range.contains(UID::new(BASE.as_u64() + 9)), true);
        assert_eq!(range.contains(UID::new(BASE.as_u64() + 10)), false);
    }

    #[test]
    fn contains_stepped() {
        let range = UIDRange::new_count(BASE, 10, 3);
        assert_eq!(range.contains(UID::new(BASE.as_u64() - 3)), false);
        assert_eq!(range.contains(UID::new(BASE.as_u64() - 1)), false);
        assert_eq!(range.contains(UID::new(BASE.as_u64() + 0)), true);
        assert_eq!(range.contains(UID::new(BASE.as_u64() + 1)), false);
        assert_eq!(range.contains(UID::new(BASE.as_u64() + 3)), true);
        assert_eq!(range.contains(UID::new(BASE.as_u64() + 27)), true);
        assert_eq!(range.contains(UID::new(BASE.as_u64() + 28)), false);
        assert_eq!(range.contains(UID::new(BASE.as_u64() + 30)), false);
    }

    #[test]
    fn index_of() {
        let range = UIDRange::new_count(BASE, 10, 1);
        assert_eq!(range.index_of(UID::new(BASE.as_u64() - 1)), None);
        assert_eq!(range.index_of(UID::new(BASE.as_u64() + 0)), Some(0));
        assert_eq!(range.index_of(UID::new(BASE.as_u64() + 9)), Some(9));
        assert_eq!(range.index_of(UID::new(BASE.as_u64() + 10)), None);
    }

    #[test]
    fn index_of_stepped() {
        let range = UIDRange::new_count(BASE, 10, 3);
        assert_eq!(range.index_of(UID::new(BASE.as_u64() - 3)), None);
        assert_eq!(range.index_of(UID::new(BASE.as_u64() - 1)), None);
        assert_eq!(range.index_of(UID::new(BASE.as_u64() + 0)), Some(0));
        assert_eq!(range.index_of(UID::new(BASE.as_u64() + 1)), None);
        assert_eq!(range.index_of(UID::new(BASE.as_u64() + 3)), Some(1));
        assert_eq!(range.index_of(UID::new(BASE.as_u64() + 27)), Some(9));
        assert_eq!(range.index_of(UID::new(BASE.as_u64() + 28)), None);
        assert_eq!(range.index_of(UID::new(BASE.as_u64() + 30)), None);
    }
}
