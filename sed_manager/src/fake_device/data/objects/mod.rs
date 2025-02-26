mod authority;
mod c_pin;
mod locking_range;
mod sp;

use super::table::Table;
use crate::spec::{
    column_types::{AuthorityRef, CPINRef, LockingRangeRef, SPRef},
    table_id,
};

pub use authority::Authority;
pub use c_pin::CPIN;
pub use locking_range::LockingRange;
pub use sp::SP;

pub type AuthorityTable = Table<Authority, AuthorityRef, { table_id::AUTHORITY.as_u64() }>;
pub type CPINTable = Table<CPIN, CPINRef, { table_id::C_PIN.as_u64() }>;
pub type SPTable = Table<SP, SPRef, { table_id::SP.as_u64() }>;
pub type LockingTable = Table<LockingRange, LockingRangeRef, { table_id::LOCKING.as_u64() }>;
