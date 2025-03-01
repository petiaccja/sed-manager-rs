#![allow(dead_code)]

mod com_id_session;
pub mod data;
mod device;
mod discovery;
mod packet_stack;
mod security_provider_session;

pub use device::FakeDevice;

pub const MSID_PASSWORD: &str = "default_password";
pub const PSID_PASSWORD: &str = "psid_password";
pub use discovery::BASE_COM_ID;
