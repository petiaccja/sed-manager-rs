use super::value::{Bytes, Value};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UID {
    table: u32,
    object: u32,
}

impl UID {
    pub const fn null() -> Self {
        Self::new(0)
    }

    pub const fn new(value: u64) -> Self {
        Self { table: (value >> 32) as u32, object: value as u32 }
    }

    pub const fn as_u64(&self) -> u64 {
        ((self.table as u64) << 32) | (self.object as u64)
    }

    pub const fn is_table(&self) -> bool {
        self.table != 0 && self.object == 0
    }

    pub const fn is_descriptor(&self) -> bool {
        self.table == 1 && self.object != 0
    }

    pub const fn is_object(&self) -> bool {
        self.object != 0
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
}

impl From<UID> for Value {
    fn from(value: UID) -> Self {
        Value::from(Bytes::from(value.as_u64().to_be_bytes()))
    }
}

impl TryFrom<Value> for UID {
    type Error = Value;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match <&Bytes>::try_from(&value) {
            Ok(bytes) => match <[u8; 8]>::try_from(bytes.as_slice()) {
                Ok(fixed_bytes) => Ok(Self::new(u64::from_be_bytes(fixed_bytes))),
                Err(_) => Err(value),
            },
            Err(_) => Err(value),
        }
    }
}

impl From<u64> for UID {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl From<UID> for u64 {
    fn from(value: UID) -> Self {
        value.as_u64()
    }
}

impl Default for UID {
    fn default() -> Self {
        UID::null()
    }
}

impl std::fmt::Debug for UID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UID( {:8x}_{:8x} )", self.table, self.object)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: UID = UID::new(0x0000_0001_0000_0000);
    const DESCRIPTOR: UID = UID::new(0x0000_0001_0000_0001);
    const OBJECT: UID = UID::new(0x0000_0009_0000_0001);
    const CONTAINING: UID = UID::new(0x0000_0009_0000_0000);
    const SM_METHOD: UID = UID::new(0x0000_0000_0000_FF01);

    #[test]
    fn determine_uid_type() {
        assert!(TABLE.is_table());
        assert!(!TABLE.is_descriptor());
        assert!(!TABLE.is_object());

        assert!(!DESCRIPTOR.is_table());
        assert!(DESCRIPTOR.is_descriptor());
        assert!(DESCRIPTOR.is_object());

        assert!(!OBJECT.is_table());
        assert!(!OBJECT.is_descriptor());
        assert!(OBJECT.is_object());

        assert!(!SM_METHOD.is_table());
        assert!(!SM_METHOD.is_descriptor());
        assert!(SM_METHOD.is_object());
    }

    #[test]
    fn convert_table_and_descriptor() {
        assert_eq!(TABLE.to_descriptor().unwrap(), DESCRIPTOR);
        assert_eq!(TABLE, DESCRIPTOR.to_table().unwrap());
    }

    #[test]
    fn get_containing_table() {
        assert_eq!(OBJECT.containing_table().unwrap(), CONTAINING);
        assert_eq!(DESCRIPTOR.containing_table().unwrap(), TABLE);
    }
}
