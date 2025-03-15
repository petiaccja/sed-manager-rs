pub mod alias_struct;
pub mod data_enum;
pub mod data_struct;
pub mod numeric_enum;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteOrder {
    Inherit,
    BigEndian,
    LittleEndian,
}
