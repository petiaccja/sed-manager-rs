//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

mod com_session;
mod control_session;
mod sp_session;
mod tper;

// `Session` is unambiguous as `ControlSession` and `ComSession` don't make sense outside.
pub use sp_session::SPSession as Session;
pub use tper::{discover, TPer};
