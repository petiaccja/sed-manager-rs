use std::sync::Arc;
use std::time::Duration;
use std::usize;

use sed_manager::fake_device::FakeDevice;
use sed_manager::messaging::types::List;
use sed_manager::rpc::{Error as RPCError, Properties};
use sed_manager::specification::sp;
use sed_manager::tper::TPer;

const HOST_PROPERTIES: Properties = Properties {
    max_methods: usize::MAX,
    max_subpackets: usize::MAX,
    max_gross_packet_size: usize::MAX,
    max_packets: usize::MAX,
    max_gross_compacket_size: 1048576,
    max_gross_compacket_response_size: 1048576,
    max_ind_token_size: 1048576 - 56,
    max_agg_token_size: 1048576 - 56,
    continued_tokens: false,
    seq_numbers: false,
    ack_nak: false,
    asynchronous: true,
    buffer_mgmt: false,
    max_retries: 3,
    trans_timeout: Duration::from_secs(15),
};

#[tokio::test]
async fn properties_with_host() -> Result<(), RPCError> {
    let device = FakeDevice::new();
    let device_caps = device.capabilities().clone();
    let tper = TPer::new(Arc::new(device));
    let (tper_properties, host_properties) = tper.properties(Some(HOST_PROPERTIES.to_list())).await?;
    let tper_properties = Properties::from_list(&tper_properties);
    let host_properties = Properties::from_list(&host_properties.unwrap_or(List::new()));
    assert_eq!(tper_properties, device_caps);
    assert_eq!(host_properties, Properties::common(&device_caps, &HOST_PROPERTIES));
    assert_eq!(tper.active_properties().await, Properties::common(&HOST_PROPERTIES, &tper_properties));
    Ok(())
}

#[tokio::test]
async fn start_session_normal() -> Result<(), RPCError> {
    let device = Arc::new(FakeDevice::new());
    {
        let tper = TPer::new(device.clone());
        let _session = tper.start_session(sp::ADMIN.try_into().unwrap()).await?;
    }
    assert!(device.active_sessions().is_empty());
    Ok(())
}
