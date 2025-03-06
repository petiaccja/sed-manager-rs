use sed_manager::messaging::com_id::ComIdState;
use sed_manager_gui_elements::{ConfigureState, ExtendedStatus, LockingRangeState, TroubleshootState, UserListState};
use slint::Model;

use crate::{
    ui,
    utility::{as_vec_model, into_vec_model},
};

pub trait StateExt {
    fn wipe(&self);

    fn respond_list_devices(
        &self,
        result: Result<(Vec<ui::DeviceIdentity>, Vec<ui::UnavailableDevice>), ui::ExtendedStatus>,
    );
    fn respond_discover(
        &self,
        device_idx: usize,
        result: Result<(ui::DeviceDiscovery, ui::ActivitySupport, ui::DeviceGeometry), ui::ExtendedStatus>,
    );
    fn respond_take_ownership(&self, device_idx: usize, status: ui::ExtendedStatus);
    fn respond_activate_locking(&self, device_idx: usize, status: ui::ExtendedStatus);
    fn respond_login_locking_admin(&self, device_idx: usize, status: ui::ExtendedStatus);
    fn push_locking_range(&self, device_idx: usize, name: String, range: ui::LockingRange);
    fn respond_set_locking_range(
        &self,
        device_idx: usize,
        range_idx: usize,
        result: Result<ui::LockingRange, ui::ExtendedStatus>,
    );
    fn respond_locking_range_status(&self, device_idx: usize, range_idx: usize, result: ui::ExtendedStatus);
    fn push_user(&self, device_idx: usize, name: String, range: ui::User);
    fn respond_set_locking_user_enabled(
        &self,
        device_idx: usize,
        user_idx: usize,
        status: Result<bool, ui::ExtendedStatus>,
    );
    fn respond_set_locking_user_name(
        &self,
        device_idx: usize,
        user_idx: usize,
        status: Result<String, ui::ExtendedStatus>,
    );
    fn respond_locking_user_status(&self, device_idx: usize, user_idx: usize, status: ui::ExtendedStatus);
    fn respond_revert(&self, device_idx: usize, status: ui::ExtendedStatus);
    fn respond_reset_stack(&self, device_idx: usize, result: ui::ExtendedStatus);
}

impl<'a> StateExt for ui::State<'a> {
    fn wipe(&self) {
        self.set_device_list_status(ui::ExtendedStatus::loading());
        self.set_tab_names(into_vec_model(vec![]));
        self.set_descriptions(into_vec_model(vec![]));
        self.set_unavailable_devices(into_vec_model(vec![]));
        self.set_configure(into_vec_model(vec![]));
        self.set_troubleshoot(into_vec_model(vec![]));
    }

    fn respond_list_devices(
        &self,
        result: Result<(Vec<ui::DeviceIdentity>, Vec<ui::UnavailableDevice>), ui::ExtendedStatus>,
    ) {
        let (identities, unavailable_devices) = match result {
            Ok(value) => value,
            Err(status) => {
                self.set_device_list_status(status);
                return;
            }
        };
        let num_devices = identities.len();
        let mut tabs: Vec<_> = identities.iter().map(|identity| identity.name.clone()).collect();
        if !unavailable_devices.is_empty() {
            tabs.push("Unavailable devices".into());
        }
        let descriptions: Vec<_> = identities
            .into_iter()
            .map(|identity| {
                ui::DeviceDescription::new(
                    identity,
                    ui::ExtendedStatus::loading(),
                    ui::DeviceDiscovery::empty(),
                    ui::ActivitySupport::none(),
                    ui::DeviceGeometry::unknown(),
                )
            })
            .collect();
        let configures: Vec<_> = std::iter::repeat_with(|| ui::ConfigureState::empty()).take(num_devices).collect();
        let troubleshoots: Vec<_> =
            std::iter::repeat_with(|| ui::TroubleshootState::empty()).take(num_devices).collect();
        self.set_device_list_status(ui::ExtendedStatus::success());
        self.set_descriptions(into_vec_model(descriptions));
        self.set_unavailable_devices(into_vec_model(unavailable_devices));
        self.set_tab_names(into_vec_model(tabs));
        self.set_configure(into_vec_model(configures));
        self.set_troubleshoot(into_vec_model(troubleshoots));
    }

    fn respond_discover(
        &self,
        device_idx: usize,
        result: Result<(ui::DeviceDiscovery, ui::ActivitySupport, ui::DeviceGeometry), ui::ExtendedStatus>,
    ) {
        let descriptions = self.get_descriptions();
        let Some(desc) = descriptions.row_data(device_idx) else {
            return;
        };
        let new_desc = match result {
            Ok((discovery, activity_support, geometry)) => ui::DeviceDescription {
                discovery_status: ui::ExtendedStatus::success(),
                discovery,
                activity_support,
                geometry,
                ..desc
            },
            Err(error) => ui::DeviceDescription { discovery_status: error, ..desc },
        };
        descriptions.set_row_data(device_idx, new_desc);
    }

    fn respond_take_ownership(&self, device_idx: usize, status: ui::ExtendedStatus) {
        respond_configure(self, device_idx, status);
    }

    fn respond_activate_locking(&self, device_idx: usize, status: ui::ExtendedStatus) {
        respond_configure(self, device_idx, status);
    }

    fn respond_login_locking_admin(&self, device_idx: usize, status: ui::ExtendedStatus) {
        respond_configure(self, device_idx, status);
    }

    fn push_locking_range(&self, device_idx: usize, name: String, range: ui::LockingRange) {
        let config = self.get_configure();
        let Some(dev_config) = config.row_data(device_idx) else {
            return;
        };
        as_vec_model(&dev_config.locking_ranges.names).push(name.into());
        as_vec_model(&dev_config.locking_ranges.properties).push(range.into());
        as_vec_model(&dev_config.locking_ranges.statuses).push(ui::ExtendedStatus::success());
        config.set_row_data(device_idx, dev_config);
    }

    fn respond_set_locking_range(
        &self,
        device_idx: usize,
        range_idx: usize,
        result: Result<ui::LockingRange, ui::ExtendedStatus>,
    ) {
        let config = self.get_configure();
        let Some(dev_config) = config.row_data(device_idx) else {
            return;
        };
        let prop_vec = as_vec_model(&dev_config.locking_ranges.properties);
        let status_vec = as_vec_model(&dev_config.locking_ranges.statuses);
        match result {
            Ok(new_props) => {
                if range_idx < prop_vec.row_count() {
                    prop_vec.set_row_data(range_idx, new_props);
                }
                if range_idx < status_vec.row_count() {
                    status_vec.set_row_data(range_idx, ExtendedStatus::success());
                }
            }
            Err(error) => {
                if range_idx < status_vec.row_count() {
                    status_vec.set_row_data(range_idx, error);
                }
            }
        }
    }

    fn respond_locking_range_status(&self, device_idx: usize, range_idx: usize, result: ui::ExtendedStatus) {
        let config = self.get_configure();
        let Some(dev_config) = config.row_data(device_idx) else {
            return;
        };
        let status_vec = as_vec_model(&dev_config.locking_ranges.statuses);
        if range_idx < status_vec.row_count() {
            status_vec.set_row_data(range_idx, result);
        }
    }

    fn push_user(&self, device_idx: usize, name: String, user: ui::User) {
        let config = self.get_configure();
        let Some(dev_config) = config.row_data(device_idx) else {
            return;
        };
        as_vec_model(&dev_config.users.names).push(name.into());
        as_vec_model(&dev_config.users.properties).push(user.into());
        as_vec_model(&dev_config.users.statuses).push(ui::ExtendedStatus::success());
        config.set_row_data(device_idx, dev_config);
    }

    fn respond_set_locking_user_enabled(
        &self,
        device_idx: usize,
        user_idx: usize,
        result: Result<bool, ui::ExtendedStatus>,
    ) {
        let config = self.get_configure();
        let Some(dev_config) = config.row_data(device_idx) else {
            return;
        };
        match result {
            Ok(enabled) => {
                if let Some(mut properties) = dev_config.users.properties.row_data(user_idx) {
                    properties.enabled = enabled;
                    dev_config.users.properties.set_row_data(user_idx, properties);
                }
                self.respond_locking_user_status(device_idx, user_idx, ui::ExtendedStatus::success());
            }
            Err(status) => {
                self.respond_locking_user_status(device_idx, user_idx, status);
            }
        }
    }

    fn respond_set_locking_user_name(
        &self,
        device_idx: usize,
        user_idx: usize,
        result: Result<String, ui::ExtendedStatus>,
    ) {
        let config = self.get_configure();
        let Some(dev_config) = config.row_data(device_idx) else {
            return;
        };
        match result {
            Ok(new_name) => {
                if let Some(mut properties) = dev_config.users.properties.row_data(user_idx) {
                    properties.friendly_name = new_name.into();
                    dev_config.users.properties.set_row_data(user_idx, properties);
                }
                self.respond_locking_user_status(device_idx, user_idx, ui::ExtendedStatus::success());
            }
            Err(status) => {
                self.respond_locking_user_status(device_idx, user_idx, status);
            }
        }
    }

    fn respond_locking_user_status(&self, device_idx: usize, user_idx: usize, status: ui::ExtendedStatus) {
        let config = self.get_configure();
        let Some(dev_config) = config.row_data(device_idx) else {
            return;
        };
        let status_vec = as_vec_model(&dev_config.users.statuses);
        if user_idx < status_vec.row_count() {
            status_vec.set_row_data(user_idx, status);
        }
    }

    fn respond_revert(&self, device_idx: usize, status: ui::ExtendedStatus) {
        respond_configure(self, device_idx, status);
    }

    fn respond_reset_stack(&self, device_idx: usize, result: ui::ExtendedStatus) {
        let troubleshoot = self.get_troubleshoot();
        if device_idx < troubleshoot.row_count() {
            let state = TroubleshootState::new(0, 0, ComIdState::Invalid, result);
            troubleshoot.set_row_data(device_idx, state);
        }
    }
}

fn respond_configure(state: &ui::State, device_idx: usize, status: ui::ExtendedStatus) {
    let configure = state.get_configure();
    let configure_vec = as_vec_model(&configure);
    if device_idx < configure_vec.row_count() {
        let value = ConfigureState::new(status, LockingRangeState::empty(), UserListState::empty());
        configure_vec.set_row_data(device_idx, value);
    }
}
