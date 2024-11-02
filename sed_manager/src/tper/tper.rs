use super::error::Error;
use std::cell::OnceCell;

use crate::{
    device::Device,
    messaging::packet::Discovery,
    serialization::{Deserialize, InputStream},
};

pub struct TPer {
    device: Box<dyn Device>,
    cached_discovery: OnceCell<Discovery>,
}

impl TPer {
    pub fn new(device: Box<dyn Device>) -> TPer {
        TPer { device: device, cached_discovery: OnceCell::new() }
    }

    pub fn take(self) -> Box<dyn Device> {
        self.device
    }

    pub fn discovery(&self) -> Result<&Discovery, Error> {
        match self.cached_discovery.get() {
            Some(discovery) => Ok(discovery),
            None => {
                let data = self.device.security_recv(0x01, 0x0001_u16.to_be_bytes(), 4096)?;
                let mut stream = InputStream::from(data);
                let discovery = Discovery::deserialize(&mut stream)?;
                Ok(self.cached_discovery.get_or_init(|| discovery))
            }
        }
    }
}
