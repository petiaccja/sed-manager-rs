use super::uid::UID;

pub struct UIDRange {
    base: UID,
    count: u64,
}

impl UIDRange {
    pub const fn new(base: UID, count: u64) -> Self {
        Self { base, count }
    }

    pub const fn n(&self, offset: u64) -> Option<UID> {
        if offset < self.count {
            Some(UID::new(self.base.value() + offset))
        } else {
            None
        }
    }
}
