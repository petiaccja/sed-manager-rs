mod com_id;
mod com_session;
mod device;
mod internal_error;
mod management_session;
mod packet_session;
mod session;
mod tper;

pub use device::{NUM_COM_IDS, VirtualDevice};
pub const BASE_COM_ID: u16 = device::BASE_COM_ID.0;
