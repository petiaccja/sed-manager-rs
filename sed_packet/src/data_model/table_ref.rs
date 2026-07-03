use std::ops::{Add, AddAssign, Sub, SubAssign};

use crate::data_model::uid::Uid;
use crate::token::{Detokenize, Detokenizer, MessageError, Tokenize, Tokenizer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TableRef(Uid);

impl TableRef {
    pub const fn try_new(value: u64) -> Option<Self> {
        let uid = Uid::new(value);
        if uid.is_table() || uid.is_null() { Some(Self(uid)) } else { None }
    }

    pub const fn new(value: u64) -> Self {
        Self::try_new(value).expect("expected a table reference or a null UID")
    }

    pub const fn containing_table(uid: Uid) -> Option<Self> {
        match uid.containing_table() {
            Some(table_uid) => Some(Self(table_uid)),
            None => None,
        }
    }

    pub const fn to_u64(&self) -> u64 {
        self.0.to_u64()
    }

    pub const fn to_uid(&self) -> Uid {
        self.0
    }

    pub const fn add(&self, offset: u32) -> Self {
        Self(self.0.next_table(offset).expect("this UID is always a table"))
    }

    pub const fn sub(&self, offset: u32) -> Self {
        Self(self.0.previous_table(offset).expect("this UID is always a table"))
    }

    pub const fn diff(&self, rhs: Self) -> i64 {
        (self.to_u64() >> 32) as i64 - (rhs.to_u64() >> 32) as i64
    }
}

impl TryFrom<u64> for TableRef {
    type Error = u64;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::try_new(value).ok_or(value)
    }
}

impl TryFrom<Uid> for TableRef {
    type Error = Uid;

    fn try_from(value: Uid) -> Result<Self, Self::Error> {
        Self::try_new(value.to_u64()).ok_or(value)
    }
}

impl From<TableRef> for u64 {
    fn from(value: TableRef) -> Self {
        value.to_u64()
    }
}

impl From<TableRef> for Uid {
    fn from(value: TableRef) -> Self {
        value.to_uid()
    }
}

impl Tokenize for TableRef {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        self.0.tokenize(tokenizer)
    }
}

impl Detokenize for TableRef {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        Self::try_from(Uid::detokenize(detokenizer)?)
            .map_err(|uid| D::Error::message(format!("expected a table reference, got UID `{uid}`")))
    }
}

impl Add<u32> for TableRef {
    type Output = Self;

    fn add(self, rhs: u32) -> Self::Output {
        Self::add(&self, rhs)
    }
}

impl Add<u32> for &TableRef {
    type Output = TableRef;

    fn add(self, rhs: u32) -> Self::Output {
        (*self).add(rhs)
    }
}

impl Add<u32> for &mut TableRef {
    type Output = TableRef;

    fn add(self, rhs: u32) -> Self::Output {
        (*self).add(rhs)
    }
}

impl AddAssign<u32> for TableRef {
    fn add_assign(&mut self, rhs: u32) {
        *self = self.add(rhs);
    }
}

impl Sub<u32> for TableRef {
    type Output = Self;

    fn sub(self, rhs: u32) -> Self::Output {
        Self::sub(&self, rhs)
    }
}

impl Sub<u32> for &TableRef {
    type Output = TableRef;

    fn sub(self, rhs: u32) -> Self::Output {
        (*self).sub(rhs)
    }
}

impl Sub<u32> for &mut TableRef {
    type Output = TableRef;

    fn sub(self, rhs: u32) -> Self::Output {
        (*self).sub(rhs)
    }
}

impl SubAssign<u32> for TableRef {
    fn sub_assign(&mut self, rhs: u32) {
        *self = self.sub(rhs);
    }
}

impl Sub for TableRef {
    type Output = i64;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::diff(&self, rhs)
    }
}

impl Sub for &TableRef {
    type Output = i64;

    fn sub(self, rhs: Self) -> Self::Output {
        *self - *rhs
    }
}

impl std::fmt::Display for TableRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    //--------------------------------------------------------------------------
    // TableRef
    //--------------------------------------------------------------------------

    #[rstest]
    #[case::null(0, true)]
    #[case::table(0x0000_0001_0000_0000, true)]
    #[case::table_desc(0x0000_0001_0000_0001, false)]
    #[case::special(0x0000_0000_0000_0056, false)]
    fn try_new_object_ref(#[case] value: u64, #[case] success: bool) {
        let object_ref = TableRef::try_new(value);
        if success {
            assert_eq!(object_ref.map(|r| r.to_u64()), Some(value));
        } else {
            assert!(object_ref.is_none());
        }
    }

    #[rstest]
    #[case(TableRef::new(0x0000_0001_0000_0000), TableRef::new(0x0000_0001_0000_0000), 0)]
    #[case(TableRef::new(0x0000_0001_0000_0000), TableRef::new(0x0000_0002_0000_0000), -1)]
    #[case(TableRef::new(0x0000_0002_0000_0000), TableRef::new(0x0000_0001_0000_0000), 1)]
    fn diff(#[case] lhs: TableRef, #[case] rhs: TableRef, #[case] expected: i64) {
        assert_eq!(lhs.diff(rhs), expected);
    }
}
