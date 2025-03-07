use std::sync::Arc;

use sed_manager::applications::get_lookup;
use sed_manager::device::{Device, Error as DeviceError};
use sed_manager::messaging::discovery::{Discovery, Feature};
use sed_manager::messaging::uid::UID;
use sed_manager::rpc::Error as RPCError;
use sed_manager::spec::column_types::{AuthorityRef, LockingRangeRef, SPRef};
use sed_manager::spec::{self, ObjectLookup as _};
use sed_manager::tper::{Session, TPer};

pub struct Backend {
    devices: Vec<Arc<dyn Device>>,
    discoveries: Vec<Option<Discovery>>,
    tpers: Vec<Option<Arc<TPer>>>,
    sessions: Vec<Option<Arc<Session>>>,
    locking_ranges: Vec<Vec<LockingRangeRef>>,
    locking_users: Vec<Vec<AuthorityRef>>,
}

impl Backend {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            discoveries: Vec::new(),
            tpers: Vec::new(),
            sessions: Vec::new(),
            locking_ranges: Vec::new(),
            locking_users: Vec::new(),
        }
    }

    pub fn set_devices(&mut self, devices: Vec<Arc<dyn Device>>) {
        let num_devices = devices.len();
        self.devices = devices;
        self.discoveries = std::iter::repeat_with(|| None).take(num_devices).collect();
        self.tpers = std::iter::repeat_with(|| None).take(num_devices).collect();
        self.sessions = std::iter::repeat_with(|| None).take(num_devices).collect();
        self.locking_ranges = std::iter::repeat_with(|| vec![]).take(num_devices).collect();
        self.locking_users = std::iter::repeat_with(|| vec![]).take(num_devices).collect();
    }

    pub fn get_device(&mut self, device_idx: usize) -> Option<Arc<dyn Device>> {
        self.devices.get(device_idx).cloned()
    }

    pub fn set_discovery(&mut self, device_idx: usize, discovery: Discovery) {
        if device_idx < self.discoveries.len() {
            self.discoveries[device_idx] = Some(discovery);
        }
    }

    pub fn get_discovery(&self, device_idx: usize) -> Result<&Discovery, RPCError> {
        self.discoveries.get(device_idx).and_then(|x| x.as_ref()).ok_or(DeviceError::DeviceNotFound.into())
    }

    pub fn get_tper(&mut self, device_idx: usize) -> Result<Arc<TPer>, RPCError> {
        let maybe_tper = self.tpers.get_mut(device_idx).ok_or(DeviceError::DeviceNotFound)?;
        if let Some(tper) = maybe_tper {
            return Ok(tper.clone());
        }
        let device = self.devices.get(device_idx).ok_or(DeviceError::DeviceNotFound)?;
        let maybe_discovery = self.discoveries.get(device_idx).ok_or(RPCError::Unspecified)?; // GUI should never allow this state.
        let discovery = maybe_discovery.as_ref().ok_or(RPCError::Unspecified)?; // GUI should never allow this state.
        let ssc = discovery.get_primary_ssc().ok_or(RPCError::NotSupported)?;
        let com_id = ssc.base_com_id();
        let com_id_ext = 0;
        let tper = Arc::new(TPer::new(device.clone(), com_id, com_id_ext));
        drop(maybe_tper.replace(tper.clone()));
        Ok(tper)
    }

    pub fn replace_session(&mut self, device_idx: usize, session: Session) -> Option<Arc<Session>> {
        if device_idx < self.devices.len() {
            self.sessions.resize_with(std::cmp::max(self.sessions.len(), device_idx + 1), || None);
            self.sessions[device_idx].replace(Arc::new(session))
        } else {
            None
        }
    }

    pub fn take_session(&mut self, device_idx: usize) -> Option<Arc<Session>> {
        self.sessions.get_mut(device_idx).map(|x| x.take()).flatten()
    }

    pub fn get_session(&self, device_idx: usize) -> Option<Arc<Session>> {
        self.sessions.get(device_idx).cloned().flatten()
    }

    pub fn set_range_list(&mut self, device_idx: usize, locking_ranges: Vec<LockingRangeRef>) {
        if device_idx < self.devices.len() {
            self.locking_ranges.resize_with(std::cmp::max(self.sessions.len(), device_idx + 1), || vec![]);
            self.locking_ranges[device_idx] = locking_ranges;
        }
    }

    pub fn get_range_list(&self, device_idx: usize) -> Option<&Vec<LockingRangeRef>> {
        self.locking_ranges.get(device_idx)
    }

    pub fn set_user_list(&mut self, device_idx: usize, locking_users: Vec<AuthorityRef>) {
        if device_idx < self.devices.len() {
            self.locking_users.resize_with(std::cmp::max(self.sessions.len(), device_idx + 1), || vec![]);
            self.locking_users[device_idx] = locking_users;
        }
    }

    pub fn get_user_list(&self, device_idx: usize) -> Option<&Vec<AuthorityRef>> {
        self.locking_users.get(device_idx)
    }
}

pub fn get_object_name(discovery: Option<&Discovery>, uid: UID, sp: Option<SPRef>) -> String {
    // Try all present feature descriptors.
    let empty = Discovery::new(vec![]);
    for desc in discovery.unwrap_or(&empty).iter() {
        let lookup = get_lookup(desc.feature_code());
        if let Some(name) = lookup.by_uid(uid, sp.map(|sp| sp.as_uid())) {
            return name;
        }
    }
    // Try features sets that don't have a feature desriptor.
    if let Some(name) = spec::psid::OBJECT_LOOKUP.by_uid(uid, sp.map(|sp| sp.as_uid())) {
        return name;
    }
    // Format the UID as a hex number.
    format!("{:16x}", uid.as_u64())
}
