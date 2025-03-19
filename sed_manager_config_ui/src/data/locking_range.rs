use crate::LockingRange;

impl LockingRange {
    pub fn new(
        start_lba: u64,
        end_lba: u64,
        read_lock_enabled: bool,
        write_lock_enabled: bool,
        read_locked: bool,
        write_locked: bool,
    ) -> Self {
        Self {
            start_lba: start_lba as i32, // Works up to 1024 TiB. 64-bit would be better.
            end_lba: end_lba as i32,     // Works up to 1024 TiB. 64-bit would be better.
            read_lock_enabled,
            write_lock_enabled,
            read_locked,
            write_locked,
        }
    }
}
