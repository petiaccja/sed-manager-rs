use std::time::Duration;

#[derive(Clone)]
pub struct Properties {
    pub max_methods: usize,
    pub max_subpackets: usize,
    pub max_gross_packet_size: usize,
    pub max_packets: usize,
    pub max_gross_compacket_size: usize,
    pub max_gross_compacket_response_size: usize,
    pub max_ind_token_size: usize,
    pub max_agg_token_size: usize,
    pub continued_tokens: bool,
    pub seq_numbers: bool,
    pub ack_nak: bool,
    pub asynchronous: bool,
    pub buffer_mgmt: bool,
    pub max_retries: u32,
    pub timeout: Duration,
}

const fn default_properties() -> Properties {
    Properties {
        max_methods: 1,
        max_subpackets: 1,
        max_gross_packet_size: 1004,
        max_packets: 1,
        max_gross_compacket_size: 1024,
        max_gross_compacket_response_size: 1024,
        max_ind_token_size: 968,
        max_agg_token_size: 968,
        continued_tokens: false,
        seq_numbers: false,
        ack_nak: false,
        asynchronous: false,
        buffer_mgmt: false,
        max_retries: 3,
        timeout: Duration::from_secs(5),
    }
}

impl Default for Properties {
    fn default() -> Self {
        default_properties()
    }
}

pub const ASSUMED_PROPERTIES: Properties = default_properties();
