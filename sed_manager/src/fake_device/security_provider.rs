use crate::messaging::uid::UID;

pub struct SecurityProvider {
    uid: UID,
}

impl SecurityProvider {
    pub fn new(uid: UID) -> Self {
        Self { uid }
    }

    pub fn uid(&self) -> UID {
        self.uid
    }
}
