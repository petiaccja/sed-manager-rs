use sed_manager_gui_slint as ui;
use sed_spec::{methods::Properties, property_name};
use slint::{ToSharedString, VecModel};

use crate::ui_conv::IntoUi;

macro_rules! display_property {
    ($name:ident, $host:expr, $device:expr, $connection:expr) => {
        ui::ProtocolProperty {
            name: property_name!($name).to_shared_string(),
            host: $host.$name.into_ui(),
            device: match $device {
                Some(device) => device.$name.into_ui(),
                None => "-".to_shared_string(),
            },
            connection: match $connection {
                Some(connection) => connection.$name.into_ui(),
                None => "-".to_shared_string(),
            },
        }
    };
}

pub struct CombinedProperties {
    pub host: Properties,
    pub device: Option<Properties>,
    pub connection: Option<Properties>,
}

impl IntoUi for CombinedProperties {
    type Ui = VecModel<ui::ProtocolProperty>;

    fn into_ui(&self) -> Self::Ui {
        let properties = vec![
            display_property!(max_methods, &self.host, &self.device, &self.connection),
            display_property!(max_methods, &self.host, &self.device, &self.connection),
            display_property!(max_subpackets, &self.host, &self.device, &self.connection),
            display_property!(max_gross_packet_size, &self.host, &self.device, &self.connection),
            display_property!(max_packets, &self.host, &self.device, &self.connection),
            display_property!(max_gross_compacket_size, &self.host, &self.device, &self.connection),
            display_property!(max_gross_compacket_response_size, &self.host, &self.device, &self.connection),
            display_property!(max_sessions, &self.host, &self.device, &self.connection),
            display_property!(max_read_sessions, &self.host, &self.device, &self.connection),
            display_property!(max_ind_token_size, &self.host, &self.device, &self.connection),
            display_property!(max_agg_token_size, &self.host, &self.device, &self.connection),
            display_property!(max_authentications, &self.host, &self.device, &self.connection),
            display_property!(max_transaction_limit, &self.host, &self.device, &self.connection),
            display_property!(def_session_timeout, &self.host, &self.device, &self.connection),
            display_property!(max_session_timeout, &self.host, &self.device, &self.connection),
            display_property!(min_session_timeout, &self.host, &self.device, &self.connection),
            display_property!(def_trans_timeout, &self.host, &self.device, &self.connection),
            display_property!(max_trans_timeout, &self.host, &self.device, &self.connection),
            display_property!(min_trans_timeout, &self.host, &self.device, &self.connection),
            display_property!(max_com_id_time, &self.host, &self.device, &self.connection),
            display_property!(continued_tokens, &self.host, &self.device, &self.connection),
            display_property!(seq_numbers, &self.host, &self.device, &self.connection),
            display_property!(ack_nak, &self.host, &self.device, &self.connection),
            display_property!(asynchronous, &self.host, &self.device, &self.connection),
        ];
        VecModel::from(properties)
    }
}
