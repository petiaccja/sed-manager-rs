//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

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
