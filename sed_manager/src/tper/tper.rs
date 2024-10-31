use std::cell::OnceCell;

use crate::{
    device,
    device::Device,
    messaging::packet::Discovery,
    serialization::{Deserialize, InputStream},
};

pub struct TPer {
    device: Box<dyn Device>,
    cached_discovery: OnceCell<Discovery>,
}



impl TPer {
    pub fn discovery(&self) -> Result<&Discovery, device::Error> {
        todo!();
        //match self.cached_discovery.get() {
            // Some(discovery) => Ok(discovery),
            // None => {
            //     let data = self.device.security_recv(0x01, 0x0001_u16.to_be_bytes(), 4096)?;
            //     let mut stream = InputStream::from(data);
            //     match discovery = Discovery::deserialize(&mut stream) {

            //     };
            //     Ok(self.cached_discovery.get_or_init(|| discovery))
            // }
        //}
    }
}
