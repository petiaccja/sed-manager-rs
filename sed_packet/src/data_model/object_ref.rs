use core::marker::PhantomData;

use crate::data_model::table_ref::TableRef;
use crate::data_model::uid::Uid;
use crate::token::{Detokenize, Detokenizer, MessageError, Tokenize, Tokenizer};

//------------------------------------------------------------------------------
// Traits
//------------------------------------------------------------------------------

pub trait Object {
    const TABLE: TableRef;
}

pub trait Field<const INDEX: u16> {
    type Type;
}

//------------------------------------------------------------------------------
// Object reference
//------------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectRef<O: Object>(Uid, PhantomData<O>);

impl<O: Object> Clone for ObjectRef<O> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<O: Object> Copy for ObjectRef<O> {}

impl<O: Object> ObjectRef<O> {
    pub const fn new(value: u64) -> Option<Self> {
        let uid = Uid::new(value);
        match uid.containing_table() {
            Some(table) if table.to_u64() == O::TABLE.to_u64() => Some(Self(uid, PhantomData)),
            _ => None,
        }
    }

    pub const fn new_unchecked(value: u64) -> Self {
        Self::new(value).expect("the UID does not refer to an object in this table")
    }

    pub const fn to_u64(&self) -> u64 {
        self.0.to_u64()
    }

    pub const fn to_uid(&self) -> Uid {
        self.0
    }
}

impl<O, const INDEX: u16> Field<INDEX> for ObjectRef<O>
where
    O: Object,
    O: Field<INDEX>,
{
    type Type = <O as Field<INDEX>>::Type;
}

impl<O: Object> TryFrom<u64> for ObjectRef<O> {
    type Error = u64;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(value)
    }
}

impl<O: Object> TryFrom<Uid> for ObjectRef<O> {
    type Error = Uid;

    fn try_from(value: Uid) -> Result<Self, Self::Error> {
        Self::new(value.to_u64()).ok_or(value)
    }
}

impl<O: Object> From<ObjectRef<O>> for u64 {
    fn from(value: ObjectRef<O>) -> Self {
        value.to_u64()
    }
}

impl<O: Object> From<ObjectRef<O>> for Uid {
    fn from(value: ObjectRef<O>) -> Self {
        value.to_uid()
    }
}

impl<O: Object> Tokenize for ObjectRef<O> {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        self.0.tokenize(tokenizer)
    }
}

impl<O: Object> Detokenize for ObjectRef<O> {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        Self::try_from(Uid::detokenize(detokenizer)?).map_err(|_| D::Error::message("the UID must refer to a table"))
    }
}

//------------------------------------------------------------------------------
// Field reference
//------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldRef<O, const FIELD: u16>(ObjectRef<O>)
where
    O: Object + Field<FIELD>;

impl<O, const FIELD: u16> FieldRef<O, FIELD>
where
    O: Object + Field<FIELD>,
{
    pub const fn new(object: ObjectRef<O>) -> Self {
        Self(object)
    }

    pub fn object(&self) -> ObjectRef<O> {
        self.0
    }

    pub fn field(&self) -> u16 {
        FIELD
    }
}

impl<O, const FIELD: u16> Field<FIELD> for FieldRef<O, FIELD>
where
    O: Object + Field<FIELD>,
{
    type Type = <O as Field<FIELD>>::Type;
}
