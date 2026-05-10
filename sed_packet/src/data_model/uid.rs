use crate::token::{Detokenize, Detokenizer, Tokenize, Tokenizer};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Uid {
    table: u32,
    object: u32,
}

impl Uid {
    pub const fn null() -> Self {
        Self::new(0)
    }

    pub const fn new(value: u64) -> Self {
        Self { table: (value >> 32) as u32, object: value as u32 }
    }

    pub const fn to_u64(&self) -> u64 {
        ((self.table as u64) << 32) | (self.object as u64)
    }

    pub const fn to_half_uid(&self) -> u32 {
        self.object
    }

    pub const fn is_table(&self) -> bool {
        self.table != 0 && self.object == 0
    }

    pub const fn is_descriptor(&self) -> bool {
        self.table == 1 && self.object != 0
    }

    pub const fn is_object(&self) -> bool {
        self.table != 0 && self.object != 0
    }

    pub const fn is_special(&self) -> bool {
        self.table == 0
    }

    pub const fn to_descriptor(&self) -> Option<Self> {
        if self.is_table() {
            Some(Self { table: 1, object: self.table })
        } else {
            None
        }
    }

    pub const fn to_table(&self) -> Option<Self> {
        if self.is_descriptor() {
            Some(Self { table: self.object, object: 0 })
        } else {
            None
        }
    }

    pub const fn containing_table(&self) -> Option<Self> {
        if self.is_object() || self.is_descriptor() {
            Some(Self { table: self.table, object: 0 })
        } else {
            None
        }
    }

    pub const fn next_object(&self, offset: u32) -> Option<Self> {
        if self.is_object() {
            let object = match self.object + offset {
                x @ 0 => x + 1, // Behave the same way as an addition.
                x => x,
            };
            Some(Self { table: self.table, object })
        } else {
            None
        }
    }

    pub const fn previous_object(&self, offset: u32) -> Option<Self> {
        if self.is_object() {
            let object = match self.object - offset {
                x @ 0 => x - 1, // Behave the same way as an underflow.
                x => x,
            };
            Some(Self { table: self.table, object })
        } else {
            None
        }
    }

    pub const fn next_table(&self, offset: u32) -> Option<Self> {
        if self.is_table() {
            let table = match self.table + offset {
                x @ 0 => x + 1, // Behave the same way as an addition.
                x => x,
            };
            Some(Self { table, object: 0 })
        } else {
            None
        }
    }

    pub const fn previous_table(&self, offset: u32) -> Option<Self> {
        if self.is_table() {
            let table = match self.table - offset {
                x @ 0 => x - 1, // Behave the same way as an underflow.
                x => x,
            };
            Some(Self { table, object: 0 })
        } else {
            None
        }
    }
}

impl Tokenize for Uid {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        tokenizer.tokenize_bytes(&self.to_u64().to_be_bytes())
    }
}

impl Detokenize for Uid {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        Ok(Self::new(u64::from_be_bytes(<[u8; 8]>::detokenize(detokenizer)?)))
    }
}

impl From<u64> for Uid {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl From<Uid> for u64 {
    fn from(value: Uid) -> Self {
        value.to_u64()
    }
}

impl Default for Uid {
    fn default() -> Self {
        Self::null()
    }
}

impl core::fmt::Display for Uid {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:#010x}_{:08x}", self.table, self.object)
    }
}

impl core::fmt::Debug for Uid {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Uid::{}", self)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    const TABLE: Uid = Uid::new(0x0000_0001_0000_0000);
    const DESCRIPTOR: Uid = Uid::new(0x0000_0001_0000_0001);
    const OBJECT: Uid = Uid::new(0x0000_0009_0000_0001);
    const SOME_TABLE: Uid = Uid::new(0x0000_0009_0000_0000);
    const SM_METHOD: Uid = Uid::new(0x0000_0000_0000_FF01);

    #[test]
    fn uid_check_uid_kind() {
        assert!(TABLE.is_table());
        assert!(!TABLE.is_descriptor());
        assert!(!TABLE.is_object());
        assert!(!TABLE.is_special());

        assert!(!DESCRIPTOR.is_table());
        assert!(DESCRIPTOR.is_descriptor());
        assert!(DESCRIPTOR.is_object());
        assert!(!DESCRIPTOR.is_special());

        assert!(!OBJECT.is_table());
        assert!(!OBJECT.is_descriptor());
        assert!(OBJECT.is_object());
        assert!(!OBJECT.is_special());

        assert!(!SM_METHOD.is_table());
        assert!(!SM_METHOD.is_descriptor());
        assert!(!SM_METHOD.is_object());
        assert!(SM_METHOD.is_special());
    }

    #[test]
    fn uid_to_descriptor() {
        assert_eq!(TABLE.to_descriptor(), Some(DESCRIPTOR));
        assert_eq!(DESCRIPTOR.to_descriptor(), None);
        assert_eq!(OBJECT.to_descriptor(), None);
        assert_eq!(SM_METHOD.to_descriptor(), None);
    }

    #[test]
    fn uid_to_table() {
        assert_eq!(TABLE.to_table(), None);
        assert_eq!(DESCRIPTOR.to_table(), Some(TABLE));
        assert_eq!(OBJECT.to_table(), None);
        assert_eq!(SM_METHOD.to_table(), None);
    }

    #[test]
    fn uid_containing_table() {
        assert_eq!(TABLE.containing_table(), None);
        assert_eq!(DESCRIPTOR.containing_table(), Some(TABLE));
        assert_eq!(OBJECT.containing_table(), Some(SOME_TABLE));
        assert_eq!(SM_METHOD.containing_table(), None);
    }

    #[rstest]
    #[case(Uid::new(0x0000_0001_0000_0001), 1, Some(Uid::new(0x0000_0001_0000_0002)))]
    #[case(Uid::new(0x0000_0001_0000_0000), 1, None)]
    fn next_object(#[case] lhs: Uid, #[case] rhs: u32, #[case] expected: Option<Uid>) {
        assert_eq!(lhs.next_object(rhs), expected);
    }

    #[rstest]
    #[case(Uid::new(0x0000_0001_0000_0003), 1, Some(Uid::new(0x0000_0001_0000_0002)))]
    #[case(Uid::new(0x0000_0001_0000_0000), 1, None)]
    fn previous_object(#[case] lhs: Uid, #[case] rhs: u32, #[case] expected: Option<Uid>) {
        assert_eq!(lhs.previous_object(rhs), expected);
    }

    #[rstest]
    #[case(Uid::new(0x0000_0001_0000_0000), 1, Some(Uid::new(0x0000_0002_0000_0000)))]
    #[case(Uid::new(0x0000_0001_0000_0001), 1, None)]
    fn next_table(#[case] lhs: Uid, #[case] rhs: u32, #[case] expected: Option<Uid>) {
        assert_eq!(lhs.next_table(rhs), expected);
    }

    #[rstest]
    #[case(Uid::new(0x0000_0003_0000_0000), 1, Some(Uid::new(0x0000_0002_0000_0000)))]
    #[case(Uid::new(0x0000_0003_0000_0001), 1, None)]
    #[case(Uid::new(0x0000_0000_0000_0000), 1, None)]
    fn previous_table(#[case] lhs: Uid, #[case] rhs: u32, #[case] expected: Option<Uid>) {
        assert_eq!(lhs.previous_table(rhs), expected);
    }
}
