use std::ops::{Add, Sub};

#[derive(Debug, Clone, Copy)]
pub struct UidRange<U> {
    pub start: U,
    pub end: U,
    pub step: u32,
}

impl<U> UidRange<U> {
    pub fn get(&self, index: usize) -> Option<U>
    where
        U: Clone + Add<u32, Output = U> + Sub<U, Output = i64>,
    {
        if index < self.len() {
            Some(self.start.clone() + index as u32 * self.step)
        } else {
            None
        }
    }

    pub fn len(&self) -> usize
    where
        U: Clone + Sub<U, Output = i64>,
    {
        match self.end.clone() - self.start.clone() {
            diff @ 0.. => diff as usize / self.step as usize,
            _ => 0,
        }
    }

    /// The inverse of [`Self::get`]: returns the index of `uid` within the range,
    /// or `None` if `uid` is out of bounds or not aligned with `step`.
    pub fn index_of(&self, uid: U) -> Option<usize>
    where
        U: Clone + PartialOrd + Sub<U, Output = i64>,
    {
        if !self.contains(uid.clone()) {
            return None;
        }
        let offset = uid - self.start.clone();
        let index = offset / self.step as i64;
        Some(usize::try_from(index).expect("only u32::MAX entries, `contains` ensures positive"))
    }

    pub fn contains(&self, uid: U) -> bool
    where
        U: Clone + PartialOrd + Sub<U, Output = i64>,
    {
        self.start <= uid && uid < self.end && (uid - self.start.clone()) % self.step as i64 == 0
    }
}

impl<U> Iterator for UidRange<U>
where
    U: Clone + Add<u32, Output = U> + PartialOrd + Sub<U, Output = i64>,
{
    type Item = U;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start < self.end {
            let item = self.start.clone();
            self.start = self.start.clone() + self.step;
            Some(item)
        } else {
            None
        }
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        if n < self.len() {
            self.start = self.start.clone() + n as u32 * self.step;
            self.next()
        } else {
            self.start = self.end.clone();
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

impl<U> ExactSizeIterator for UidRange<U>
where
    Self: Iterator,
    U: Clone + Sub<U, Output = i64>,
{
    fn len(&self) -> usize {
        Self::len(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::ObjectRef;

    use super::*;

    const START: ObjectRef<0x0000_0001_0000_0000> = ObjectRef::new(0x0000_0001_0000_0001);
    const END: ObjectRef<0x0000_0001_0000_0000> = ObjectRef::new(0x0000_0001_0000_0005);
    const END_LONG: ObjectRef<0x0000_0001_0000_0000> = ObjectRef::new(0x0000_0001_0000_000F);

    #[test]
    fn iter_next_one() {
        let mut range = UidRange { start: START, end: END, step: 1 };
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
        let mut range = UidRange { start: START, end: END, step: 2 };
        assert_eq!(range.next(), Some(START));
        assert_eq!(range.next(), Some(START + 2));
        assert!(range.len() == 0);
        assert_eq!(range.next(), None);
        assert_eq!(range.next(), None);
    }

    #[test]
    fn iter_nth_one() {
        let mut range = UidRange { start: START, end: END, step: 1 };
        assert_eq!(range.nth(2), Some(START + 2));
        assert!(range.len() == 1);
        assert_eq!(range.nth(2), None);
        assert!(range.len() == 0);
        assert_eq!(range.nth(2), None);
    }

    #[test]
    fn iter_nth_stepped() {
        // Tip: use the regular Range and its nth implementation to check for proper behaviour.
        let mut range = UidRange { start: START, end: END_LONG, step: 2 };
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
            let range = UidRange { start: START, end: START, step: 1 };
            assert_eq!(range.len(), 0);
        }
        {
            let range = UidRange { start: START, end: START + 12, step: 1 };
            assert_eq!(range.len(), 12);
        }
        {
            let range = UidRange { start: START, end: START + 12, step: 2 };
            assert_eq!(range.len(), 6);
        }
        {
            let range = UidRange { start: START, end: START + 11, step: 2 };
            assert_eq!(range.len(), 5);
        }
    }

    #[test]
    fn index_of_in_bounds() {
        let range = UidRange { start: START, end: END_LONG, step: 2 };
        assert_eq!(range.index_of(START), Some(0));
        assert_eq!(range.index_of(START + 2), Some(1));
        assert_eq!(range.index_of(START + 4), Some(2));
    }

    #[test]
    fn index_of_before_start() {
        let range = UidRange { start: START + 4, end: END_LONG, step: 2 };
        assert_eq!(range.index_of(START), None);
    }

    #[test]
    fn index_of_at_end_excluded() {
        let range = UidRange { start: START, end: END, step: 1 };
        assert_eq!(range.index_of(END), None);
    }

    #[test]
    fn index_of_misaligned_with_step() {
        let range = UidRange { start: START, end: END_LONG, step: 2 };
        assert_eq!(range.index_of(START + 1), None);
    }

    #[test]
    fn index_of_agrees_with_get() {
        let range = UidRange { start: START, end: END_LONG, step: 2 };
        for index in 0..range.len() {
            let uid = range.get(index).unwrap();
            assert_eq!(range.index_of(uid), Some(index));
        }
    }
}
