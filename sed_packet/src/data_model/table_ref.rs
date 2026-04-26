use crate::data_model::uid::Uid;
use crate::token::{Detokenize, Detokenizer, MessageError, Tokenize, Tokenizer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TableRef(Uid);

impl TableRef {
    pub const fn new(value: u64) -> Option<Self> {
        let uid = Uid::new(value);
        if uid.is_table() { Some(Self(uid)) } else { None }
    }

    pub const fn containing_table(uid: Uid) -> Option<Self> {
        match uid.containing_table() {
            Some(table_uid) => Some(Self(table_uid)),
            None => None,
        }
    }

    pub const fn new_unchecked(value: u64) -> Self {
        Self::new(value).expect("the UID must refer to a table")
    }

    pub const fn to_u64(&self) -> u64 {
        self.0.to_u64()
    }

    pub const fn to_uid(&self) -> Uid {
        self.0
    }
}

impl TryFrom<u64> for TableRef {
    type Error = u64;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(value)
    }
}

impl TryFrom<Uid> for TableRef {
    type Error = Uid;

    fn try_from(value: Uid) -> Result<Self, Self::Error> {
        Self::new(value.to_u64()).ok_or(value)
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
        Self::try_from(Uid::detokenize(detokenizer)?).map_err(|_| D::Error::message("the UID must refer to a table"))
    }
}

impl std::fmt::Display for TableRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
