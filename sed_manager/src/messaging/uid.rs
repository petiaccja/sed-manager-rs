use super::value::{Bytes, Value};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UID {
    table: u32,
    object: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TableUID(UID);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectUID<const TABLE_MASK: u64>(UID);

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

    pub const fn as_uid(&self) -> UID {
        *self
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
}

impl TableUID {
    const fn is_value_accepted(value: u64) -> bool {
        let value_uid = UID::new(value);
        value_uid.is_table()
    }

    pub const fn new(value: u64) -> Self {
        assert!(Self::is_value_accepted(value));
        Self(UID::new(value))
    }

    pub const fn try_new(value: u64) -> Result<Self, u64> {
        if Self::is_value_accepted(value) {
            Ok(Self(UID::new(value)))
        } else {
            Err(value)
        }
    }

    pub const fn as_u64(&self) -> u64 {
        self.0.as_u64()
    }

    pub const fn as_uid(&self) -> UID {
        self.0
    }

    pub const fn mask(&self) -> u64 {
        super::table_mask::table_mask(*self)
    }

    pub const fn is_contained_in_mask(&self, mask: u64) -> bool {
        super::table_mask::is_table_in_mask(*self, mask)
    }

    pub const fn to_descriptor(&self) -> ObjectUID<1> {
        ObjectUID(self.0.to_descriptor().unwrap())
    }
}

impl<const TABLE_MASK: u64> ObjectUID<TABLE_MASK> {
    const fn is_value_accepted(value: u64) -> bool {
        let value_uid = UID::new(value);
        match value_uid.containing_table() {
            Some(table) => TableUID::new(table.as_u64()).is_contained_in_mask(TABLE_MASK),
            None => false,
        }
    }

    pub const fn new(value: u64) -> Self {
        assert!(Self::is_value_accepted(value));
        Self(UID::new(value))
    }

    pub const fn try_new(value: u64) -> Result<Self, u64> {
        if Self::is_value_accepted(value) {
            Ok(Self(UID::new(value)))
        } else {
            Err(value)
        }
    }

    pub const fn new_other<const OTHER_TABLE_MASK: u64>(other: ObjectUID<OTHER_TABLE_MASK>) -> Self {
        Self::new(other.as_u64())
    }

    pub const fn try_new_other<const OTHER_TABLE_MASK: u64>(
        other: ObjectUID<OTHER_TABLE_MASK>,
    ) -> Result<Self, ObjectUID<OTHER_TABLE_MASK>> {
        match Self::try_new(other.as_u64()) {
            Ok(value) => Ok(value),
            Err(_) => Err(other),
        }
    }

    pub const fn as_u64(&self) -> u64 {
        self.0.as_u64()
    }

    pub const fn as_uid(&self) -> UID {
        self.0
    }

    pub const fn is_descriptor(&self) -> bool {
        self.0.is_descriptor()
    }

    pub const fn to_table(&self) -> Option<TableUID> {
        if self.is_descriptor() {
            Some(TableUID(UID { table: self.0.object, object: 0 }))
        } else {
            None
        }
    }

    pub const fn containing_table(&self) -> TableUID {
        TableUID::new(self.0.containing_table().unwrap().as_u64())
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
        match <&[u8; 8]>::try_from(&value) {
            Ok(bytes) => Ok(Self::new(u64::from_be_bytes(*bytes))),
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
        Self::null()
    }
}

impl From<TableUID> for Value {
    fn from(value: TableUID) -> Self {
        Value::from(value.as_uid())
    }
}

impl TryFrom<Value> for TableUID {
    type Error = Value;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match <&[u8; 8]>::try_from(&value) {
            Ok(bytes) => match Self::try_new(u64::from_be_bytes(*bytes)) {
                Ok(uid) => Ok(uid),
                Err(_) => Err(value),
            },
            Err(_) => Err(value),
        }
    }
}

impl TryFrom<u64> for TableUID {
    type Error = u64;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl From<TableUID> for u64 {
    fn from(value: TableUID) -> Self {
        value.as_u64()
    }
}

impl TryFrom<UID> for TableUID {
    type Error = UID;
    fn try_from(value: UID) -> Result<Self, Self::Error> {
        Self::try_new(value.as_u64()).map_err(|_| value)
    }
}

impl From<TableUID> for UID {
    fn from(value: TableUID) -> Self {
        value.0
    }
}

impl<const TABLE_MASK: u64> From<ObjectUID<TABLE_MASK>> for Value {
    fn from(value: ObjectUID<TABLE_MASK>) -> Self {
        Value::from(value.as_uid())
    }
}

impl<const TABLE_MASK: u64> TryFrom<Value> for ObjectUID<TABLE_MASK> {
    type Error = Value;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match <&[u8; 8]>::try_from(&value) {
            Ok(bytes) => match Self::try_new(u64::from_be_bytes(*bytes)) {
                Ok(uid) => Ok(uid),
                Err(_) => Err(value),
            },
            Err(_) => Err(value),
        }
    }
}

impl<const TABLE_MASK: u64> TryFrom<u64> for ObjectUID<TABLE_MASK> {
    type Error = u64;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl<const TABLE_MASK: u64> From<ObjectUID<TABLE_MASK>> for u64 {
    fn from(value: ObjectUID<TABLE_MASK>) -> Self {
        value.as_u64()
    }
}

impl<const TABLE_MASK: u64> TryFrom<UID> for ObjectUID<TABLE_MASK> {
    type Error = UID;
    fn try_from(value: UID) -> Result<Self, Self::Error> {
        Self::try_new(value.as_u64()).map_err(|_| value)
    }
}

impl<const TABLE_MASK: u64> From<ObjectUID<TABLE_MASK>> for UID {
    fn from(value: ObjectUID<TABLE_MASK>) -> Self {
        value.0
    }
}

impl core::fmt::Debug for UID {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "UID::{:#010x}_{:08x}", self.table, self.object)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: UID = UID::new(0x0000_0001_0000_0000);
    const DESCRIPTOR: UID = UID::new(0x0000_0001_0000_0001);
    const OBJECT: UID = UID::new(0x0000_0009_0000_0001);
    const SOME_TABLE: UID = UID::new(0x0000_0009_0000_0000);
    const SM_METHOD: UID = UID::new(0x0000_0000_0000_FF01);

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

    #[test]
    fn table_uid_try_new() {
        assert_eq!(TableUID::try_from(TABLE), Ok(TableUID::new(TABLE.as_u64())));
        assert_eq!(TableUID::try_from(DESCRIPTOR), Err(DESCRIPTOR));
        assert_eq!(TableUID::try_from(OBJECT), Err(OBJECT));
        assert_eq!(TableUID::try_from(SM_METHOD), Err(SM_METHOD));
    }

    #[test]
    fn object_uid_try_new() {
        type SomeTableUID = ObjectUID<{ TableUID::new(SOME_TABLE.as_u64()).mask() }>;
        assert_eq!(SomeTableUID::try_from(TABLE), Err(TABLE));
        assert_eq!(SomeTableUID::try_from(DESCRIPTOR), Err(DESCRIPTOR));
        assert_eq!(SomeTableUID::try_from(OBJECT), Ok(SomeTableUID::new(OBJECT.as_u64())));
        assert_eq!(SomeTableUID::try_from(SM_METHOD), Err(SM_METHOD));
    }
}
