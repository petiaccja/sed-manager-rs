//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_manager_macros::StructType;

use super::{
    define_column_type,
    enumeration_types::{Day, Month, Year},
};

define_column_type!(Date, 0x0000_0005_0000_1804_u64, "date");

#[derive(StructType, PartialEq, Eq, Clone, Debug, Default)]
pub struct Date {
    year: Year,
    month: Month,
    day: Day,
}
