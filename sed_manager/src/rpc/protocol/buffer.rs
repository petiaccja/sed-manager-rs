use std::sync::atomic::{AtomicU32, Ordering};

pub struct Buffer {
    capacity: u32,
    used: AtomicU32,
}

impl Buffer {
    pub fn new(capacity: u32) -> Self {
        Self { capacity, used: 0.into() }
    }

    pub fn used(&self) -> u32 {
        self.used.load(Ordering::Relaxed)
    }

    pub fn allocate(&self, size: u32) -> bool {
        let result = self.used.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |used| {
            if size + used <= self.capacity {
                Some(size + used)
            } else {
                None
            }
        });
        result.is_ok()
    }

    pub fn deallocate(&self, size: u32) {
        let result = self.used.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |used| {
            if used >= size {
                Some(used - size)
            } else {
                None
            }
        });
        if result.is_err() {
            panic!("deallocating more than what's currently in use");
        };
    }

    pub fn deallocate_all(&self) -> u32 {
        self.used.swap(0, Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_success() {
        let buffer = Buffer::new(100);
        assert!(buffer.allocate(35));
        assert_eq!(buffer.used(), 35);
    }

    #[test]
    fn allocate_failure() {
        let buffer = Buffer::new(100);
        assert!(!buffer.allocate(110));
        assert_eq!(buffer.used(), 0);
    }

    #[test]
    fn deallocate_normal() {
        let buffer = Buffer::new(100);
        buffer.used.store(70, Ordering::Relaxed);
        buffer.deallocate(30);
        assert_eq!(buffer.used(), 40);
    }

    #[test]
    #[should_panic]
    fn deallocate_panic() {
        let buffer = Buffer::new(100);
        buffer.used.store(70, Ordering::Relaxed);
        buffer.deallocate(80);
    }

    #[test]
    fn deallocate_all() {
        let buffer = Buffer::new(100);
        buffer.used.store(70, Ordering::Relaxed);
        assert_eq!(buffer.deallocate_all(), 70);
    }
}
