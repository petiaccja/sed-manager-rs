//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

mod com_id_session;
// mod control_session;
// mod control_session_dispatcher;
pub mod data;
mod device;
mod discovery;
mod tper;
// mod packet_stack;
// mod sp_session;
// mod sp_session_dispatcher;
mod dispatch;
mod protocol_stack;

pub use device::FakeDevice;

pub const MSID_PASSWORD: &str = "default_password";
pub const PSID_PASSWORD: &str = "psid_password";
pub use data::god_authority;
pub use discovery::BASE_COM_ID;
