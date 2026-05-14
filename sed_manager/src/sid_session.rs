use sed_packet::MaxBytes;
use sed_tper::Tper;

use crate::error::Error;

pub struct SidSession {
    tper: Tper,
}

impl SidSession {
    pub fn take_owneship(&self, new_sid_password: MaxBytes<32>) -> Result<(), Error> {
        todo!()
    }
}
