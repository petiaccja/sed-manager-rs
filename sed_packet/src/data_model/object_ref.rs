use core::marker::PhantomData;
use std::ops::{Add, AddAssign, Sub, SubAssign};

use crate::data_model::table_ref::TableRef;
use crate::data_model::uid::Uid;
use crate::token::{Detokenize, Detokenizer, MessageError, Tokenize, Tokenizer};

//------------------------------------------------------------------------------
// Traits
//------------------------------------------------------------------------------

pub trait Object {
    const TABLE: TableRef;
    type Ref;
    const FIELD_COUNT: u16;

    fn active_fields(&self) -> Vec<u16>;
    fn update(&mut self, other: Self);
}

pub trait Field<const INDEX: u16> {
    type Type;
}

pub trait ObjectUid: Object {
    fn uid(&self) -> Option<Self::Ref>;
}

//------------------------------------------------------------------------------
// Object reference
//------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectRef<const TABLE: u64>(Uid);

impl<const TABLE: u64> ObjectRef<TABLE> {
    pub const fn try_new(value: u64) -> Option<Self> {
        let uid = Uid::new(value);
        match uid.containing_table() {
            Some(table) if table.to_u64() == TABLE => Some(Self(uid)),
            _ => None,
        }
    }

    pub const fn new(value: u64) -> Self {
        Self::try_new(value).expect("the UID does not refer to an object in this table")
    }

    pub const fn to_u64(&self) -> u64 {
        self.0.to_u64()
    }

    pub const fn to_uid(&self) -> Uid {
        self.0
    }

    pub const fn to_half_uid(&self) -> u32 {
        self.0.to_half_uid()
    }

    pub const fn add(&self, offset: u32) -> Self {
        Self(self.0.next_object(offset).expect("this UID is always an object"))
    }

    pub const fn sub(&self, offset: u32) -> Self {
        Self(self.0.previous_object(offset).expect("this UID is always an object"))
    }

    pub const fn diff(&self, rhs: Self) -> i64 {
        (self.to_u64() & 0xFFFF_FFFF) as i64 - (rhs.to_u64() & 0xFFFF_FFFF) as i64
    }
}

impl<const TABLE: u64> TryFrom<u64> for ObjectRef<TABLE> {
    type Error = u64;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::try_new(value).ok_or(value)
    }
}

impl<const TABLE: u64> TryFrom<Uid> for ObjectRef<TABLE> {
    type Error = Uid;

    fn try_from(value: Uid) -> Result<Self, Self::Error> {
        Self::try_new(value.to_u64()).ok_or(value)
    }
}

impl<const TABLE: u64> From<ObjectRef<TABLE>> for u64 {
    fn from(value: ObjectRef<TABLE>) -> Self {
        value.to_u64()
    }
}

impl<const TABLE: u64> From<ObjectRef<TABLE>> for Uid {
    fn from(value: ObjectRef<TABLE>) -> Self {
        value.to_uid()
    }
}

impl<const TABLE: u64> Tokenize for ObjectRef<TABLE> {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        self.0.tokenize(tokenizer)
    }
}

impl<const TABLE: u64> Detokenize for ObjectRef<TABLE> {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        Self::try_from(Uid::detokenize(detokenizer)?).map_err(|_| D::Error::message("the UID must refer to a table"))
    }
}
impl<const TABLE: u64> Add<u32> for ObjectRef<TABLE> {
    type Output = Self;

    fn add(self, rhs: u32) -> Self::Output {
        Self(self.0.next_object(rhs).expect("this must be an object"))
    }
}

impl<const TABLE: u64> Add<u32> for &ObjectRef<TABLE> {
    type Output = ObjectRef<TABLE>;

    fn add(self, rhs: u32) -> Self::Output {
        (*self).add(rhs)
    }
}

impl<const TABLE: u64> Add<u32> for &mut ObjectRef<TABLE> {
    type Output = ObjectRef<TABLE>;

    fn add(self, rhs: u32) -> Self::Output {
        (*self).add(rhs)
    }
}

impl<const TABLE: u64> AddAssign<u32> for ObjectRef<TABLE> {
    fn add_assign(&mut self, rhs: u32) {
        *self = self.add(rhs);
    }
}

impl<const TABLE: u64> Sub<u32> for ObjectRef<TABLE> {
    type Output = Self;

    fn sub(self, rhs: u32) -> Self::Output {
        Self(self.0.previous_object(rhs).expect("this must be an object"))
    }
}

impl<const TABLE: u64> Sub<u32> for &ObjectRef<TABLE> {
    type Output = ObjectRef<TABLE>;

    fn sub(self, rhs: u32) -> Self::Output {
        (*self).sub(rhs)
    }
}

impl<const TABLE: u64> Sub<u32> for &mut ObjectRef<TABLE> {
    type Output = ObjectRef<TABLE>;

    fn sub(self, rhs: u32) -> Self::Output {
        (*self).sub(rhs)
    }
}

impl<const TABLE: u64> SubAssign<u32> for ObjectRef<TABLE> {
    fn sub_assign(&mut self, rhs: u32) {
        *self = self.sub(rhs);
    }
}

impl<const TABLE: u64> Sub for ObjectRef<TABLE> {
    type Output = i64;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::diff(&self, rhs)
    }
}

impl<const TABLE: u64> Sub for &ObjectRef<TABLE> {
    type Output = i64;

    fn sub(self, rhs: Self) -> Self::Output {
        *self - *rhs
    }
}

impl<const TABLE: u64> std::fmt::Display for ObjectRef<TABLE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0)
    }
}

//------------------------------------------------------------------------------
// Field reference
//------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldRef<O, const TABLE: u64, const FIELD: u16>(pub ObjectRef<TABLE>, PhantomData<O>);

impl<O, const TABLE: u64, const FIELD: u16> FieldRef<O, TABLE, FIELD> {
    pub const fn object(&self) -> ObjectRef<TABLE> {
        self.0
    }

    pub const fn field(&self) -> u16 {
        FIELD
    }
}

impl<O, const TABLE: u64, const FIELD: u16> From<ObjectRef<TABLE>> for FieldRef<O, TABLE, FIELD> {
    fn from(value: ObjectRef<TABLE>) -> Self {
        Self(value, PhantomData)
    }
}

impl<O, const TABLE: u64, const FIELD: u16> From<FieldRef<O, TABLE, FIELD>> for ObjectRef<TABLE> {
    fn from(value: FieldRef<O, TABLE, FIELD>) -> Self {
        value.0
    }
}

impl<O, const TABLE: u64, const FIELD: u16> std::fmt::Debug for FieldRef<O, TABLE, FIELD> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FieldRef").field("object", &self.0).field("field", &FIELD).finish()
    }
}

impl<O, const TABLE: u64, const FIELD: u16> std::fmt::Display for FieldRef<O, TABLE, FIELD> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", &self.0, &FIELD)
    }
}

impl<O, const TABLE: u64, const FIELD: u16> Field<FIELD> for FieldRef<O, TABLE, FIELD>
where
    O: Field<FIELD>,
{
    type Type = O::Type;
}
