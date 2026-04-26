mod admin;
mod locking;
mod opal_2;
mod preconfig_shared;
mod security_provider;

pub use admin::Admin;
pub use locking::Locking;
pub use opal_2::Opal2TPer;

pub enum TPer {
    Opal2(Opal2TPer),
}

impl TPer {
    pub fn pop_discovery(&mut self) -> Vec<u8> {
        todo!()
    }
}
