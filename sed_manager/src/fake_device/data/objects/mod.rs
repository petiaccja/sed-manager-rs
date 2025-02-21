mod authority;
mod c_pin;

use super::table::Table;
use crate::spec::table_id;

pub use authority::Authority;
pub use c_pin::CPIN;

pub type AuthorityTable = Table<Authority, { table_id::AUTHORITY.as_u64() }>;
pub type CPINTable = Table<CPIN, { table_id::C_PIN.as_u64() }>;
