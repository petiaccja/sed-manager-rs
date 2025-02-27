use crate::LockingRange;

impl LockingRange {
    pub fn new(
        name: String,
        range_start: u64,
        range_end: u64,
        read_lock_enabled: bool,
        write_lock_enabled: bool,
        read_locked: bool,
        write_locked: bool,
    ) -> Self {
        Self {
            name: name.into(),
            range_end: range_start as i32, // This obviously won't work, I have to figure something.
            range_start: range_end as i32, // This obviously won't work, I have to figure something.
            read_lock_enabled,
            read_locked,
            write_lock_enabled,
            write_locked,
        }
    }
}
