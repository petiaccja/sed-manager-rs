use std::rc::Rc;
use std::sync::Arc;

use sed_manager::applications::{
    activate_locking, get_admin_sp, get_locking_admins, get_locking_sp, revert, take_ownership, Error as AppError,
};
use sed_manager::device::{Device, Error as DeviceError};
use sed_manager::messaging::discovery::Discovery;
use sed_manager::rpc::{discover, Error as RPCError};
use sed_manager::spec;
use sed_manager::tper::{Session, TPer};

use crate::device_list::{get_device_identity, DeviceList};
use crate::ui;
use crate::utility::{run_in_thread, PeekCell};

pub struct Backend {
    devices: Vec<Arc<dyn Device>>,
    discoveries: Vec<Option<Discovery>>,
    tpers: Vec<Option<Arc<TPer>>>,
    sessions: Vec<Option<Session>>,
}

impl Backend {
    pub fn new() -> Self {
        Self { devices: Vec::new(), discoveries: Vec::new(), tpers: Vec::new(), sessions: Vec::new() }
    }

    #[allow(unused)]
    fn get_discovery(&mut self, device_idx: usize) -> Result<&Discovery, RPCError> {
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

    fn replace_session(&mut self, device_idx: usize, session: Session) -> Option<Session> {
        self.sessions.resize_with(std::cmp::max(self.sessions.len(), device_idx + 1), || None);
        self.sessions[device_idx].replace(session)
    }

    fn take_session(&mut self, device_idx: usize) -> Option<Session> {
        self.sessions.get_mut(device_idx).map(|x| x.take()).flatten()
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
    ) -> Result<(ui::DeviceDiscovery, ui::ActivitySupport), ui::ExtendedStatus> {
        let Some(device) = this.peek(|this| this.devices.get(device_idx).cloned()) else {
            return Err(ui::ExtendedStatus::error(format!("device {device_idx} not found (this is a bug)")));
        };
        let discovery = match run_in_thread(move || discover(&*device)).await {
            Ok(value) => value,
            Err(error) => return Err(ui::ExtendedStatus::error(error.to_string())),
        };
        let ui_discovery = ui::DeviceDiscovery::from_discovery(&discovery);
        let ui_activity_support = ui::ActivitySupport::from_discovery(&discovery);
        this.peek_mut(|this| this.discoveries.get_mut(device_idx).map(|opt| opt.replace(discovery)));
        Ok((ui_discovery, ui_activity_support))
    }

    pub async fn cleanup_session(this: Rc<PeekCell<Self>>, device_idx: usize) {
        let Some(session) = this.peek_mut(|this| this.take_session(device_idx)) else {
            return;
        };
        let _ = session.end_session().await;
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

    pub async fn login_locking_ranges(
        this: Rc<PeekCell<Self>>,
        device_idx: usize,
        admin1_pw: String,
    ) -> ui::ExtendedStatus {
        async fn inner(this: Rc<PeekCell<Backend>>, device_idx: usize, admin1_pw: String) -> Result<(), AppError> {
            let tper = this.peek_mut(|this| this.get_tper(device_idx))?;
            let discovery = tper.discover().await?;
            let ssc = discovery.get_primary_ssc().ok_or(AppError::NoAvailableSSC)?;
            let sp = get_locking_sp(ssc.feature_code())?;
            let admin1 = get_locking_admins(ssc.feature_code())?.nth(1).unwrap();
            let session = tper.start_session(sp, Some(admin1), Some(admin1_pw.as_bytes())).await?;
            this.peek_mut(|this| this.replace_session(device_idx, session));
            Ok(())
        }

        Backend::cleanup_session(this.clone(), device_idx).await;
        match inner(this, device_idx, admin1_pw).await {
            Ok(_) => ui::ExtendedStatus::success(),
            Err(error) => ui::ExtendedStatus::error(error.to_string()),
        }
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
}
