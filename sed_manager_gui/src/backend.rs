use std::rc::Rc;
use std::sync::Arc;

use sed_manager::applications::{
    activate_locking, get_admin_sp, get_locking_admins, get_locking_sp, get_lookup, revert, take_ownership,
    Error as AppError,
};
use sed_manager::device::{Device, Error as DeviceError};
use sed_manager::messaging::com_id::StackResetStatus;
use sed_manager::messaging::discovery::{Discovery, Feature};
use sed_manager::messaging::uid::UID;
use sed_manager::rpc::{discover, Error as RPCError, MethodStatus};
use sed_manager::spec::column_types::{CredentialRef, MediaKeyRef, SPRef};
use sed_manager::spec::{self, ObjectLookup as _};
use sed_manager::tper::{Session, TPer};

use crate::async_state::{AsyncState, StateExt};
use crate::device_list::{get_device_identity, DeviceList};
use crate::native_data::NativeLockingRange;
use crate::ui;
use crate::utility::{run_in_thread, PeekCell};

pub struct Backend {
    devices: Vec<Arc<dyn Device>>,
    discoveries: Vec<Option<Discovery>>,
    tpers: Vec<Option<Arc<TPer>>>,
    sessions: Vec<Option<Arc<Session>>>,
    locking_ranges: Vec<Vec<UID>>,
}

impl Backend {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            discoveries: Vec::new(),
            tpers: Vec::new(),
            sessions: Vec::new(),
            locking_ranges: Vec::new(),
        }
    }

    fn get_discovery(&self, device_idx: usize) -> Result<&Discovery, RPCError> {
        self.discoveries.get(device_idx).and_then(|x| x.as_ref()).ok_or(DeviceError::DeviceNotFound.into())
    }

    fn get_tper(&mut self, device_idx: usize) -> Result<Arc<TPer>, RPCError> {
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

    fn replace_session(&mut self, device_idx: usize, session: Session) -> Option<Arc<Session>> {
        if device_idx < self.devices.len() {
            self.sessions.resize_with(std::cmp::max(self.sessions.len(), device_idx + 1), || None);
            self.sessions[device_idx].replace(Arc::new(session))
        } else {
            None
        }
    }

    fn take_session(&mut self, device_idx: usize) -> Option<Arc<Session>> {
        self.sessions.get_mut(device_idx).map(|x| x.take()).flatten()
    }

    fn get_session(&self, device_idx: usize) -> Option<Arc<Session>> {
        self.sessions.get(device_idx).cloned().flatten()
    }

    fn set_locking_ranges(&mut self, device_idx: usize, locking_ranges: Vec<UID>) {
        if device_idx < self.devices.len() {
            self.locking_ranges.resize_with(std::cmp::max(self.sessions.len(), device_idx + 1), || vec![]);
            self.locking_ranges[device_idx] = locking_ranges;
        }
    }

    fn get_locking_ranges(&self, device_idx: usize) -> Option<&Vec<UID>> {
        self.locking_ranges.get(device_idx)
    }

    pub async fn list_devices(
        this: Rc<PeekCell<Self>>,
    ) -> Result<(Vec<ui::DeviceIdentity>, Vec<ui::UnavailableDevice>), ui::ExtendedStatus> {
        this.peek_mut(|this| {
            this.devices.clear();
            this.discoveries.clear();
            this.tpers.clear();
            this.sessions.clear();
        });
        let device_list = match DeviceList::query().await {
            Ok(value) => value,
            Err(error) => return Err(ui::ExtendedStatus::error(error.to_string())),
        };
        let mut identities = Vec::<ui::DeviceIdentity>::new();
        for device in &device_list.devices {
            identities.push(get_device_identity(device.clone()).await.into());
        }
        let unavailable_devices = device_list
            .unavailable_devices
            .into_iter()
            .map(|(path, error)| ui::UnavailableDevice::new(path, error.to_string()))
            .collect();
        this.peek_mut(move |this| {
            let num_devices = device_list.devices.len();
            this.devices = device_list.devices;
            this.discoveries = std::iter::repeat_with(|| None).take(num_devices).collect();
            this.tpers = std::iter::repeat_with(|| None).take(num_devices).collect();
            this.sessions = std::iter::repeat_with(|| None).take(num_devices).collect();
        });
        Ok((identities, unavailable_devices))
    }

    pub async fn discover(
        this: Rc<PeekCell<Self>>,
        device_idx: usize,
    ) -> Result<(ui::DeviceDiscovery, ui::ActivitySupport, ui::DeviceGeometry), ui::ExtendedStatus> {
        let Some(device) = this.peek(|this| this.devices.get(device_idx).cloned()) else {
            return Err(ui::ExtendedStatus::error(format!("device {device_idx} not found (this is a bug)")));
        };
        let discovery = match run_in_thread(move || discover(&*device)).await {
            Ok(value) => value,
            Err(error) => return Err(ui::ExtendedStatus::error(error.to_string())),
        };
        let ui_discovery = ui::DeviceDiscovery::from_discovery(&discovery);
        let ui_activity_support = ui::ActivitySupport::from_discovery(&discovery);
        let ui_geometry = ui::DeviceGeometry::from_discovery(&discovery);
        this.peek_mut(|this| this.discoveries.get_mut(device_idx).map(|opt| opt.replace(discovery)));
        Ok((ui_discovery, ui_activity_support, ui_geometry))
    }

    pub async fn cleanup_session(this: Rc<PeekCell<Self>>, device_idx: usize) {
        let Some(session) = this.peek_mut(|this| this.take_session(device_idx)) else {
            return;
        };
        if let Some(inner) = Arc::into_inner(session) {
            let _ = inner.end_session().await;
        }
    }

    pub async fn take_ownership(this: Rc<PeekCell<Self>>, device_idx: usize, sid_pw: String) -> ui::ExtendedStatus {
        async fn inner(this: Rc<PeekCell<Backend>>, device_idx: usize, sid_pw: String) -> Result<(), AppError> {
            let tper = this.peek_mut(|this| this.get_tper(device_idx))?;
            take_ownership(&*tper, sid_pw.as_bytes()).await
        }

        Backend::cleanup_session(this.clone(), device_idx).await;
        match inner(this, device_idx, sid_pw).await {
            Ok(_) => ui::ExtendedStatus::success(),
            Err(error) => ui::ExtendedStatus::error(error.to_string()),
        }
    }

    pub async fn activate_locking(
        this: Rc<PeekCell<Self>>,
        device_idx: usize,
        sid_pw: String,
        admin1_pw: String,
    ) -> ui::ExtendedStatus {
        async fn inner(
            this: Rc<PeekCell<Backend>>,
            device_idx: usize,
            sid_pw: String,
            admin1_pw: String,
        ) -> Result<(), AppError> {
            let tper = this.peek_mut(|this| this.get_tper(device_idx))?;
            activate_locking(&*tper, sid_pw.as_bytes(), Some(admin1_pw.as_bytes())).await
        }

        Backend::cleanup_session(this.clone(), device_idx).await;
        match inner(this, device_idx, sid_pw, admin1_pw).await {
            Ok(_) => ui::ExtendedStatus::success(),
            Err(error) => ui::ExtendedStatus::error(error.to_string()),
        }
    }

    pub async fn login_locking_admin(
        this: Rc<PeekCell<Self>>,
        device_idx: usize,
        admin_idx: usize,
        password: String,
    ) -> ui::ExtendedStatus {
        async fn inner(
            this: Rc<PeekCell<Backend>>,
            device_idx: usize,
            admin_idx: usize,
            password: String,
        ) -> Result<(), AppError> {
            let tper = this.peek_mut(|this| this.get_tper(device_idx))?;
            let discovery = tper.discover().await?;
            let ssc = discovery.get_primary_ssc().ok_or(AppError::NoAvailableSSC)?;
            let sp = get_locking_sp(ssc.feature_code())?;
            let Some(admin) = get_locking_admins(ssc.feature_code())?.nth(admin_idx as u64) else {
                return Err(RPCError::from(MethodStatus::InvalidParameter).into());
            };
            let session = tper.start_session(sp, Some(admin), Some(password.as_bytes())).await?;
            this.peek_mut(|this| this.replace_session(device_idx, session));
            Ok(())
        }

        Backend::cleanup_session(this.clone(), device_idx).await;
        match inner(this, device_idx, admin_idx, password).await {
            Ok(_) => ui::ExtendedStatus::success(),
            Err(error) => ui::ExtendedStatus::error(error.to_string()),
        }
    }

    pub async fn list_locking_ranges(
        this: Rc<PeekCell<Self>>,
        device_idx: usize,
        async_state: AsyncState<Rc<PeekCell<Self>>>,
    ) {
        async fn inner(
            this: Rc<PeekCell<Backend>>,
            device_idx: usize,
            async_state: AsyncState<Rc<PeekCell<Backend>>>,
        ) -> Result<(), RPCError> {
            let Some(session) = this.peek_mut(|this| this.get_session(device_idx)) else {
                return Err(RPCError::Unspecified);
            };
            let discovery = this.peek(|this| this.get_discovery(device_idx).ok().cloned());
            let ssc = discovery.as_ref().and_then(|x| x.get_primary_ssc());
            let locking_sp = ssc.and_then(|ssc| get_locking_sp(ssc.feature_code()).ok());
            let ranges = session.next(spec::core::table_id::LOCKING, None, None).await?;
            this.peek_mut(|this| this.set_locking_ranges(device_idx, ranges.clone()));
            for range in ranges.iter() {
                let name = get_object_name(discovery.as_ref(), *range, locking_sp);
                let properties = get_locking_range_properties(&session, *range).await;
                match properties {
                    Ok(properties) => {
                        async_state.with(|state| state.push_locking_range(device_idx, name, properties.into()));
                    }
                    Err(_error) => {
                        // TODO: display broken ranges as well?
                        // It's very unlikely that one range would work but another wouldn't.
                    }
                }
            }
            Ok(())
        }
        let result = inner(this, device_idx, async_state.clone()).await;
        async_state.with(|state| match result {
            Ok(_) => (),
            Err(error) => state.respond_login_locking_admin(device_idx, error.into()),
        });
    }

    pub async fn set_locking_range(
        this: Rc<PeekCell<Self>>,
        device_idx: usize,
        range_idx: usize,
        properties: ui::LockingRange,
    ) -> Result<ui::LockingRange, ui::ExtendedStatus> {
        let Some(range) = this.peek(|this| this.get_locking_ranges(device_idx).and_then(|r| r.get(range_idx).cloned()))
        else {
            return Err(RPCError::Unspecified.into());
        };
        let Some(session) = this.peek_mut(|this| this.get_session(device_idx)) else {
            return Err(RPCError::Unspecified.into());
        };
        match set_locking_range_properties(&session, range, properties.clone().into()).await {
            Ok(_) => Ok(properties),
            Err(error) => Err(error.into()),
        }
    }

    pub async fn erase_locking_range(
        this: Rc<PeekCell<Self>>,
        device_idx: usize,
        range_idx: usize,
    ) -> ui::ExtendedStatus {
        let Some(range) = this.peek(|this| this.get_locking_ranges(device_idx).and_then(|r| r.get(range_idx).cloned()))
        else {
            return RPCError::Unspecified.into();
        };
        let Some(session) = this.peek_mut(|this| this.get_session(device_idx)) else {
            return RPCError::Unspecified.into();
        };
        let result = erase_locking_range(&session, range).await;
        ui::ExtendedStatus::from_result(result)
    }

    pub async fn revert(
        this: Rc<PeekCell<Self>>,
        device_idx: usize,
        use_psid: bool,
        pw: String,
        revert_admin: bool,
    ) -> ui::ExtendedStatus {
        use spec::core::authority::SID;
        use spec::psid::admin::authority::PSID;
        async fn inner(
            this: Rc<PeekCell<Backend>>,
            device_idx: usize,
            use_psid: bool,
            pw: String,
            revert_admin: bool,
        ) -> Result<(), AppError> {
            let tper = this.peek_mut(|this| this.get_tper(device_idx))?;
            let discovery = tper.discover().await?;
            let ssc = discovery.get_primary_ssc().ok_or(AppError::NoAvailableSSC)?;
            let admin_sp = get_admin_sp(ssc.feature_code())?;
            let locking_sp = get_locking_sp(ssc.feature_code())?;
            let authority = if use_psid { PSID } else { SID };
            let sp = if revert_admin { admin_sp } else { locking_sp };
            revert(&*tper, authority, pw.as_bytes(), sp).await
        }

        Backend::cleanup_session(this.clone(), device_idx).await;
        match inner(this, device_idx, use_psid, pw, revert_admin).await {
            Ok(_) => ui::ExtendedStatus::success(),
            Err(error) => ui::ExtendedStatus::error(error.to_string()),
        }
    }

    pub async fn reset_stack(this: Rc<PeekCell<Self>>, device_idx: usize) -> ui::ExtendedStatus {
        async fn inner(this: Rc<PeekCell<Backend>>, device_idx: usize) -> Result<(), RPCError> {
            let tper = this.peek_mut(|this| this.get_tper(device_idx))?;
            let status = tper.stack_reset(tper.com_id(), tper.com_id_ext()).await?;
            match status {
                StackResetStatus::Success => Ok(()),
                StackResetStatus::Failure => Err(RPCError::Unspecified),
                StackResetStatus::Pending => Ok(()),
            }
        }
        Backend::cleanup_session(this.clone(), device_idx).await;
        let result = inner(this, device_idx).await;
        ui::ExtendedStatus::from_result(result)
    }
}

fn get_object_name(discovery: Option<&Discovery>, uid: UID, sp: Option<SPRef>) -> String {
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

async fn get_locking_range_properties(session: &Session, range: UID) -> Result<NativeLockingRange, RPCError> {
    let (start_lba, length_lba, read_lock_enabled, write_lock_enabled, read_locked, write_locked) =
        session.get_multiple::<(u64, u64, bool, bool, bool, bool)>(range, 3..=8).await?;

    Ok(NativeLockingRange {
        start_lba,
        end_lba: start_lba + length_lba,
        read_lock_enabled,
        write_lock_enabled,
        read_locked,
        write_locked,
    })
}

async fn set_locking_range_properties(
    session: &Session,
    range: UID,
    value: NativeLockingRange,
) -> Result<(), RPCError> {
    if range != spec::opal::locking::locking::GLOBAL_RANGE.as_uid() {
        let length_lba = value.end_lba - value.start_lba;
        let values = (
            value.start_lba,
            length_lba,
            value.read_lock_enabled,
            value.write_lock_enabled,
            value.read_locked,
            value.write_locked,
        );
        session.set_multiple(range, [3, 4, 5, 6, 7, 8], values).await
    } else {
        let values = (value.read_lock_enabled, value.write_lock_enabled, value.read_locked, value.write_locked);
        session.set_multiple(range, [5, 6, 7, 8], values).await
    }
}

async fn erase_locking_range(session: &Session, range: UID) -> Result<(), RPCError> {
    let active_key_id: MediaKeyRef = session.get(range, 0x0A).await?;
    session.gen_key(CredentialRef::new_other(active_key_id), None, None).await
}
