//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

pub mod basic_types;
pub mod column_types;
mod generated;
mod lookup;
pub mod objects;

pub use lookup::ObjectLookup;

// Core.
pub mod core {
    pub use super::generated::core::all::*;
}

// Feature sets.
pub use generated::data_store;
pub use generated::psid;

// Security subsystem classes.
pub use generated::enterprise;
pub use generated::kpio;
pub use generated::opal_2 as opal; // Hopefully superset of v1.0.
pub use generated::opalite;
pub use generated::pyrite_2 as pyrite; // Hopefully superset of v1.0.
pub use generated::ruby;

// Commonly used items from core.
pub use generated::core::all::invoking_id;
pub use generated::core::all::method_id;
pub use generated::core::all::sm_method_id;
pub use generated::core::all::table_id;
