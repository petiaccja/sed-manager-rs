use std::rc::Rc;

use sed_manager_gui_slint as ui;
use slint::VecModel;

pub trait DeviceExt {
    fn with_status(self, outcome: ui::Outcome, message: String) -> Self;
    fn with_identity(self, identity: ui::Identity) -> Self;
    fn with_discovery(self, discovery: ui::Discovery) -> Self;
    fn with_config(self, discovery: ui::DeviceConfig) -> Self;
    fn with_stack_status(self, stack_status: ui::StackStatus) -> Self;
}

impl DeviceExt for ui::Device {
    fn with_status(self, outcome: ui::Outcome, message: String) -> Self {
        Self { status: ui::Status { outcome, message: message.into() }, ..self }
    }

    fn with_identity(self, identity: ui::Identity) -> Self {
        Self { identity, ..self }
    }

    fn with_discovery(self, discovery: ui::Discovery) -> Self {
        Self { discovery, ..self }
    }

    fn with_config(self, config: ui::DeviceConfig) -> Self {
        Self { config, ..self }
    }

    fn with_stack_status(self, stack_status: ui::StackStatus) -> Self {
        Self { stack_status, ..self }
    }
}

pub trait DiscoveryExt {
    fn error(message: String) -> Self;
}

impl DiscoveryExt for ui::Discovery {
    fn error(message: String) -> Self {
        Self { status: ui::Status { message: message.into(), outcome: ui::Outcome::Error }, ..Default::default() }
    }
}

pub trait StackStatusExt {
    fn with_com_id(self, com_id: ui::ComIdStatus) -> Self;
    fn with_protocol(self, com_id: VecModel<ui::ProtocolProperty>) -> Self;
}

impl StackStatusExt for ui::StackStatus {
    fn with_com_id(self, com_id: ui::ComIdStatus) -> Self {
        Self { com_id, ..self }
    }

    fn with_protocol(self, protocol: VecModel<ui::ProtocolProperty>) -> Self {
        Self { protocol: Rc::from(protocol).into(), ..self }
    }
}
