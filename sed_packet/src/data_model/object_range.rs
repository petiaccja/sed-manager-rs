use core::ops::{Bound, RangeBounds};

use crate::ObjectRef;

#[derive(Debug, Clone, Copy)]
pub struct ObjectRange<const TABLE: u64> {
    pub start: ObjectRef<TABLE>,
    pub end: ObjectRef<TABLE>,
    pub step: u32,
}

impl<const TABLE: u64> ObjectRange<TABLE> {
    pub const fn get(&self, index: usize) -> Option<ObjectRef<TABLE>> {
        if index < self.len() {
            Some(self.start.add(index as u32 * self.step))
        } else {
            None
        }
    }

    pub const fn len(&self) -> usize {
        match self.end.diff(self.start) {
            diff @ 0.. => diff as usize / self.step as usize,
            _ => 0,
        }
    }
}

impl<const TABLE: u64> RangeBounds<ObjectRef<TABLE>> for ObjectRange<TABLE> {
    fn start_bound(&self) -> Bound<&ObjectRef<TABLE>> {
        Bound::Included(&self.start)
    }

    fn end_bound(&self) -> Bound<&ObjectRef<TABLE>> {
        Bound::Excluded(&self.end)
    }
}

impl<const TABLE: u64> Iterator for ObjectRange<TABLE> {
    type Item = ObjectRef<TABLE>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start < self.end {
            let item = self.start;
            self.start = self.start.clone() + self.step;
            Some(item)
        } else {
            None
        }
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        if n < self.len() {
            self.start += n as u32 * self.step;
            self.next()
        } else {
            self.start = self.end;
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let exact_size = self.len();
        (exact_size, Some(exact_size))
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.len()
    }
}

impl<const TABLE: u64> ExactSizeIterator for ObjectRange<TABLE> {
    fn len(&self) -> usize {
        self.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const START: ObjectRef<0x0000_0001_0000_0000> = ObjectRef::new(0x0000_0001_0000_0001);
    const END: ObjectRef<0x0000_0001_0000_0000> = ObjectRef::new(0x0000_0001_0000_0005);
    const END_LONG: ObjectRef<0x0000_0001_0000_0000> = ObjectRef::new(0x0000_0001_0000_000F);

    #[test]
    fn iter_next_one() {
        let mut range = ObjectRange { start: START, end: END, step: 1 };
        assert_eq!(range.next(), Some(START));
        assert_eq!(range.next(), Some(START + 1));
        assert_eq!(range.next(), Some(START + 2));
        assert_eq!(range.next(), Some(START + 3));
        assert!(range.len() == 0);
        assert_eq!(range.next(), None);
        assert_eq!(range.next(), None);
    }
    #[test]
    fn iter_next_stepped() {
        let mut range = ObjectRange { start: START, end: END, step: 2 };
        assert_eq!(range.next(), Some(START));
        assert_eq!(range.next(), Some(START + 2));
        assert!(range.len() == 0);
        assert_eq!(range.next(), None);
        assert_eq!(range.next(), None);
    }

    #[test]
    fn iter_nth_one() {
        let mut range = ObjectRange { start: START, end: END, step: 1 };
        assert_eq!(range.nth(2), Some(START + 2));
        assert!(range.len() == 1);
        assert_eq!(range.nth(2), None);
        assert!(range.len() == 0);
        assert_eq!(range.nth(2), None);
    }

    #[test]
    fn iter_nth_stepped() {
        // Tip: use the regular Range and its nth implementation to check for proper behaviour.
        let mut range = ObjectRange { start: START, end: END_LONG, step: 2 };
        assert_eq!(range.nth(2), Some(START + 4));
        assert_eq!(range.nth(2), Some(START + 10));
        assert!(range.len() == 1);
        assert_eq!(range.nth(2), None);
        assert!(range.len() == 0);
        assert_eq!(range.nth(2), None);
    }

    #[test]
    fn iter_len() {
        {
            let range = ObjectRange { start: START, end: START, step: 1 };
            assert_eq!(range.len(), 0);
        }
        {
            let range = ObjectRange { start: START, end: START + 12, step: 1 };
            assert_eq!(range.len(), 12);
        }
        {
            let range = ObjectRange { start: START, end: START + 12, step: 2 };
            assert_eq!(range.len(), 6);
        }
        {
            let range = ObjectRange { start: START, end: START + 11, step: 2 };
            assert_eq!(range.len(), 5);
        }
    }
}
