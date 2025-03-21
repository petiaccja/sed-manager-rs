use std::sync::Arc;

use sed_manager::applications::{
    get_lookup, Error as AppError, MBREditSession, PermissionEditSession, RangeEditSession, UserEditSession,
};
use sed_manager::device::{Device, Error as DeviceError};
use sed_manager::messaging::discovery::{Discovery, Feature};
use sed_manager::messaging::uid::UID;
use sed_manager::rpc::{Error as RPCError, TokioRuntime};
use sed_manager::spec::column_types::{AuthorityRef, LockingRangeRef, SPRef};
use sed_manager::spec::{self, ObjectLookup as _};
use sed_manager::tper::TPer;

pub struct Backend {
    devices: Vec<Arc<dyn Device>>,
    discoveries: Vec<Option<Discovery>>,
    tpers: Vec<Option<Arc<TPer>>>,
    sessions: Vec<Option<EditorSession>>,
    runtime: Arc<TokioRuntime>, // Has to be dropped after all TPer's are dropped.
}

pub enum EditorSession {
    Range { session: Arc<RangeEditSession>, ranges: Vec<LockingRangeRef> },
    User { session: Arc<UserEditSession>, users: Vec<AuthorityRef> },
    MBR { session: Arc<MBREditSession> },
    Permission { session: Arc<PermissionEditSession>, matrix: (Vec<AuthorityRef>, Vec<LockingRangeRef>) },
}

impl EditorSession {
    pub async fn end(self) -> Result<(), AppError> {
        match self {
            EditorSession::Range { session, ranges: _ } => {
                if let Some(inner) = Arc::into_inner(session) {
                    inner.end().await
                } else {
                    Ok(())
                }
            }
            EditorSession::User { session, users: _ } => {
                if let Some(inner) = Arc::into_inner(session) {
                    inner.end().await
                } else {
                    Ok(())
                }
            }
            EditorSession::MBR { session } => {
                if let Some(inner) = Arc::into_inner(session) {
                    inner.end().await
                } else {
                    Ok(())
                }
            }
            EditorSession::Permission { session, matrix: _ } => {
                if let Some(inner) = Arc::into_inner(session) {
                    inner.end().await
                } else {
                    Ok(())
                }
            }
        }
    }
}

impl From<RangeEditSession> for EditorSession {
    fn from(value: RangeEditSession) -> Self {
        Self::Range { session: Arc::new(value), ranges: vec![] }
    }
}

impl From<UserEditSession> for EditorSession {
    fn from(value: UserEditSession) -> Self {
        Self::User { session: Arc::new(value), users: vec![] }
    }
}

impl From<MBREditSession> for EditorSession {
    fn from(value: MBREditSession) -> Self {
        Self::MBR { session: Arc::new(value) }
    }
}

impl From<PermissionEditSession> for EditorSession {
    fn from(value: PermissionEditSession) -> Self {
        Self::Permission { session: Arc::new(value), matrix: (Vec::new(), Vec::new()) }
    }
}

impl Backend {
    pub fn new() -> Self {
        Self {
            runtime: Arc::new(TokioRuntime::new()),
            devices: Vec::new(),
            discoveries: Vec::new(),
            tpers: Vec::new(),
            sessions: Vec::new(),
        }
    }

    pub fn set_devices(&mut self, devices: Vec<Arc<dyn Device>>) {
        let num_devices = devices.len();
        self.devices = devices;
        self.discoveries = core::iter::repeat_with(|| None).take(num_devices).collect();
        self.tpers = core::iter::repeat_with(|| None).take(num_devices).collect();
        self.sessions = core::iter::repeat_with(|| None).take(num_devices).collect();
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
        let tper = Arc::new(TPer::new(device.clone(), self.runtime.clone(), com_id, com_id_ext));
        drop(maybe_tper.replace(tper.clone()));
        Ok(tper)
    }

    pub fn replace_session(&mut self, device_idx: usize, session: EditorSession) -> Option<EditorSession> {
        if device_idx < self.devices.len() {
            self.sessions.resize_with(core::cmp::max(self.sessions.len(), device_idx + 1), || None);
            self.sessions[device_idx].replace(session)
        } else {
            None
        }
    }

    pub fn take_session(&mut self, device_idx: usize) -> Option<EditorSession> {
        self.sessions.get_mut(device_idx).map(|x| x.take()).flatten()
    }

    pub fn get_session(&self, device_idx: usize) -> Option<&EditorSession> {
        self.sessions.get(device_idx).and_then(|x| x.as_ref())
    }

    pub fn get_session_mut(&mut self, device_idx: usize) -> Option<&mut EditorSession> {
        self.sessions.get_mut(device_idx).and_then(|x| x.as_mut())
    }

    pub fn get_range_session(&self, device_idx: usize) -> Result<Arc<RangeEditSession>, AppError> {
        match self.get_session(device_idx) {
            Some(EditorSession::Range { session, ranges: _ }) => Ok(session.clone()),
            _ => Err(AppError::InternalError),
        }
    }

    pub fn set_range_list(&mut self, device_idx: usize, new_ranges: Vec<LockingRangeRef>) -> Result<(), AppError> {
        match self.get_session_mut(device_idx) {
            Some(EditorSession::Range { session: _, ranges }) => {
                *ranges = new_ranges;
                Ok(())
            }
            _ => Err(AppError::InternalError),
        }
    }

    pub fn get_range_list(&self, device_idx: usize) -> Result<&[LockingRangeRef], AppError> {
        match self.get_session(device_idx) {
            Some(EditorSession::Range { session: _, ranges }) => Ok(ranges.as_slice()),
            _ => Err(AppError::InternalError),
        }
    }

    pub fn get_user_session(&self, device_idx: usize) -> Result<Arc<UserEditSession>, AppError> {
        match self.get_session(device_idx) {
            Some(EditorSession::User { session, users: _ }) => Ok(session.clone()),
            _ => Err(AppError::InternalError),
        }
    }

    pub fn set_user_list(&mut self, device_idx: usize, new_users: Vec<AuthorityRef>) -> Result<(), AppError> {
        match self.get_session_mut(device_idx) {
            Some(EditorSession::User { session: _, users }) => {
                *users = new_users;
                Ok(())
            }
            _ => Err(AppError::InternalError),
        }
    }

    pub fn get_user_list(&self, device_idx: usize) -> Result<&[AuthorityRef], AppError> {
        match self.get_session(device_idx) {
            Some(EditorSession::User { session: _, users }) => Ok(users.as_slice()),
            _ => Err(AppError::InternalError),
        }
    }

    pub fn get_mbr_session(&self, device_idx: usize) -> Result<Arc<MBREditSession>, AppError> {
        match self.get_session(device_idx) {
            Some(EditorSession::MBR { session }) => Ok(session.clone()),
            _ => Err(AppError::InternalError),
        }
    }

    pub fn get_permission_session(&self, device_idx: usize) -> Result<Arc<PermissionEditSession>, AppError> {
        match self.get_session(device_idx) {
            Some(EditorSession::Permission { session, matrix: _ }) => Ok(session.clone()),
            _ => Err(AppError::InternalError),
        }
    }

    pub fn get_permission_matrix(&self, device_idx: usize) -> Result<(&[AuthorityRef], &[LockingRangeRef]), AppError> {
        match self.get_session(device_idx) {
            Some(EditorSession::Permission { session: _, matrix }) => Ok((&matrix.0, &matrix.1)),
            _ => Err(AppError::InternalError),
        }
    }

    pub fn set_permission_matrix(
        &mut self,
        device_idx: usize,
        new_matrix: (Vec<AuthorityRef>, Vec<LockingRangeRef>),
    ) -> Result<(), AppError> {
        match self.get_session_mut(device_idx) {
            Some(EditorSession::Permission { session: _, matrix }) => {
                *matrix = new_matrix;
                Ok(())
            }
            _ => Err(AppError::InternalError),
        }
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
