mod authority;
mod c_pin;

use super::Table;
use crate::specification::tables;

pub use authority::Authority;
pub use c_pin::CPin;

pub type AuthorityTable = Table<Authority, { tables::AUTHORITY.value() }>;
pub type CPinTable = Table<CPin, { tables::C_PIN.value() }>;
