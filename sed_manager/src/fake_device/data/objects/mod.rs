mod authority;
mod c_pin;

use super::Table;
use crate::spec::table_id;

pub use authority::Authority;
pub use c_pin::CPin;

pub type AuthorityTable = Table<Authority, { table_id::AUTHORITY.as_u64() }>;
pub type CPinTable = Table<CPin, { table_id::C_PIN.as_u64() }>;
