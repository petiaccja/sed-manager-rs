use crate::messaging::types::SPRef;

pub struct SecurityProvider {
    uid: SPRef,
}

impl SecurityProvider {
    pub fn new(uid: SPRef) -> Self {
        Self { uid }
    }

    pub fn uid(&self) -> SPRef {
        self.uid
    }
}
