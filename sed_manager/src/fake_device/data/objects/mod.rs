mod authority;
mod c_pin;

use super::Table;
use crate::specification::table;

pub use authority::Authority;
pub use c_pin::CPin;

pub type AuthorityTable = Table<Authority, { table::AUTHORITY.value() }>;
pub type CPinTable = Table<CPin, { table::C_PIN.value() }>;
