use std::rc::Rc;
use std::sync::Arc;

use sed_manager::device::Device;
use sed_manager::messaging::discovery::{Discovery, Feature, FeatureCode};
use sed_manager::rpc::Error as RPCError;
use sed_manager::tper::discover;
use slint::{Model, ModelRc, ToSharedString, VecModel};

use crate::app_state::AppState;
use crate::atomic_borrow::AtomicBorrow;
use crate::generated::{ContentStatus, FeatureModel, SummaryModel};
use crate::utility::run_in_thread;

async fn get_discovery(device: Arc<dyn Device>) -> Result<Discovery, RPCError> {
    run_in_thread(move || discover(&*device)).await
}

fn get_security_providers(ssc: &FeatureCode) -> Vec<&str> {
    match ssc {
        FeatureCode::Enterprise => vec!["Admin", "Locking"],
        FeatureCode::OpalV1 => vec!["Admin", "Locking"],
        FeatureCode::OpalV2 => vec!["Admin", "Locking"],
        FeatureCode::Opalite => vec!["Admin", "Locking"],
        FeatureCode::PyriteV1 => vec!["Admin", "Locking"],
        FeatureCode::PyriteV2 => vec!["Admin", "Locking"],
        FeatureCode::Ruby => vec!["Admin", "Locking"],
        FeatureCode::KeyPerIO => vec!["Admin", "KeyPerIO"],
        _ => vec![],
    }
}

fn append_discovery(summary: SummaryModel, discovery: Result<Discovery, RPCError>) -> SummaryModel {
    match discovery {
        Ok(discovery) => {
            let common_features: Vec<_> = discovery
                .descriptors
                .iter()
                .filter(|desc| desc.security_subsystem_class().is_none())
                .map(|desc| FeatureModel::from(desc))
                .collect();
            let ssc_features: Vec<_> = discovery
                .descriptors
                .iter()
                .filter(|desc| desc.security_subsystem_class().is_some())
                .map(|desc| FeatureModel::from(desc))
                .collect();
            let ssc: Vec<_> = discovery
                .descriptors
                .iter()
                .filter(|desc| desc.security_subsystem_class().is_some())
                .map(|desc| desc.feature_code())
                .collect();
            let sp = ssc.first().map(|ssc| get_security_providers(&ssc)).unwrap_or(vec![]);

            let sp = sp.into_iter().map(|x| x.into()).collect::<Vec<_>>();
            let ssc = ssc.into_iter().map(|x| x.to_shared_string()).collect::<Vec<_>>();

            SummaryModel {
                discovery_status: ContentStatus::Success,
                security_subsystem_classes: Rc::new(VecModel::from(ssc)).into(),
                security_providers: Rc::new(VecModel::from(sp)).into(),
                common_features: Rc::new(VecModel::from(common_features)).into(),
                ssc_features: Rc::new(VecModel::from(ssc_features)).into(),
                ..summary
            }
        }
        Err(err) => SummaryModel {
            discovery_status: ContentStatus::Error,
            security_subsystem_classes: ModelRc::new(VecModel::from(vec!["None".into()])),
            discovery_error: err.to_shared_string(),
            ..summary
        },
    }
}

fn update_summary(
    app_state: Rc<AtomicBorrow<AppState>>,
    device_idx: usize,
    device: Arc<dyn Device>,
    discovery: Result<Discovery, RPCError>,
) {
    app_state.with_mut(|app_state| {
        let Some(current_device) = app_state.device_list.active_devices.get(device_idx) else {
            return;
        };
        if !Arc::ptr_eq(&current_device, &device) {
            return;
        };
        let Some(summary) = app_state.summaries.row_data(device_idx) else {
            return;
        };
        let new_summary = append_discovery(summary, discovery);
        app_state.summaries.set_row_data(device_idx, new_summary);
    });
}

pub async fn update_device_discovery(app_state: Rc<AtomicBorrow<AppState>>, device_idx: usize) {
    let Some(device) = app_state.with(|app_state| app_state.device_list.active_devices.get(device_idx).cloned()) else {
        return;
    };
    let discovery = get_discovery(device.clone()).await;
    update_summary(app_state, device_idx, device, discovery);
}
