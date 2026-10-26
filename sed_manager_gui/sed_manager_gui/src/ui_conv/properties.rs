//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_manager_gui_slint as ui;
use sed_spec::methods::Properties;
use sed_spec::property_name;
use slint::{ToSharedString, VecModel};

use crate::ui_conv::{FromBackend, IntoUi};

macro_rules! display_property {
    ($name:ident, $host:expr, $device:expr, $connection:expr) => {
        ui::ProtocolProperty {
            name: property_name!($name).to_shared_string(),
            host: (&$host.$name).into_ui(),
            device: match $device {
                Some(device) => (&device.$name).into_ui(),
                None => "-".to_shared_string(),
            },
            connection: match $connection {
                Some(connection) => (&connection.$name).into_ui(),
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

impl FromBackend<&CombinedProperties> for VecModel<ui::ProtocolProperty> {
    fn from_backend(value: &CombinedProperties) -> Self {
        let properties = vec![
            display_property!(max_methods, &value.host, &value.device, &value.connection),
            display_property!(max_methods, &value.host, &value.device, &value.connection),
            display_property!(max_subpackets, &value.host, &value.device, &value.connection),
            display_property!(max_gross_packet_size, &value.host, &value.device, &value.connection),
            display_property!(max_packets, &value.host, &value.device, &value.connection),
            display_property!(max_gross_compacket_size, &value.host, &value.device, &value.connection),
            display_property!(max_gross_compacket_response_size, &value.host, &value.device, &value.connection),
            display_property!(max_sessions, &value.host, &value.device, &value.connection),
            display_property!(max_read_sessions, &value.host, &value.device, &value.connection),
            display_property!(max_ind_token_size, &value.host, &value.device, &value.connection),
            display_property!(max_agg_token_size, &value.host, &value.device, &value.connection),
            display_property!(max_authentications, &value.host, &value.device, &value.connection),
            display_property!(max_transaction_limit, &value.host, &value.device, &value.connection),
            display_property!(def_session_timeout, &value.host, &value.device, &value.connection),
            display_property!(max_session_timeout, &value.host, &value.device, &value.connection),
            display_property!(min_session_timeout, &value.host, &value.device, &value.connection),
            display_property!(def_trans_timeout, &value.host, &value.device, &value.connection),
            display_property!(max_trans_timeout, &value.host, &value.device, &value.connection),
            display_property!(min_trans_timeout, &value.host, &value.device, &value.connection),
            display_property!(max_com_id_time, &value.host, &value.device, &value.connection),
            display_property!(continued_tokens, &value.host, &value.device, &value.connection),
            display_property!(seq_numbers, &value.host, &value.device, &value.connection),
            display_property!(ack_nak, &value.host, &value.device, &value.connection),
            display_property!(asynchronous, &value.host, &value.device, &value.connection),
        ];
        VecModel::from(properties)
    }
}
