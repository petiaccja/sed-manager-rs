use std::{time::Duration, usize};

use crate::messaging::types::{List, MaxBytes32, NamedValue};

#[derive(Clone, PartialEq, Eq, Debug)]
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
    pub trans_timeout: Duration,
}

impl Properties {
    pub const ASSUMED: Properties = Properties {
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
        trans_timeout: Duration::from_secs(5),
    };

    pub fn to_list(&self) -> List<NamedValue<MaxBytes32, u32>> {
        let list = vec![
            ("MaxMethods", inf_to_zero(self.max_methods) as u32),
            ("MaxSubpackets", inf_to_zero(self.max_subpackets) as u32),
            ("MaxPacketSize", inf_to_zero(self.max_gross_packet_size) as u32),
            ("MaxPackets", inf_to_zero(self.max_packets) as u32),
            ("MaxComPacketSize", inf_to_zero(self.max_gross_compacket_size) as u32),
            ("MaxResponseComPacketSize", inf_to_zero(self.max_gross_compacket_response_size) as u32),
            ("MaxIndTokenSize", inf_to_zero(self.max_ind_token_size) as u32),
            ("MaxAggTokenSize", inf_to_zero(self.max_agg_token_size) as u32),
            ("ContinuedTokens", self.continued_tokens as u32),
            ("SequenceNumbers", self.seq_numbers as u32),
            ("AckNak", self.ack_nak as u32),
            ("Asynchronous", self.asynchronous as u32),
            ("DefTransTimeout", self.trans_timeout.as_millis() as u32),
        ];
        list.into_iter()
            .map(|(name, value)| NamedValue { name: name.into(), value: value })
            .collect::<Vec<_>>()
            .into()
    }

    pub fn from_list(properties: &[NamedValue<MaxBytes32, u32>]) -> Self {
        let mut parsed = Properties::ASSUMED;
        for named_value in properties {
            let name = named_value.name.as_slice();
            let value = named_value.value;
            if name == "MaxMethods".as_bytes() {
                parsed.max_methods = zero_to_inf(value as usize);
            } else if name == "MaxSubpackets".as_bytes() {
                parsed.max_subpackets = zero_to_inf(value as usize);
            } else if name == "MaxPacketSize".as_bytes() {
                parsed.max_gross_packet_size = zero_to_inf(value as usize);
            } else if name == "MaxPackets".as_bytes() {
                parsed.max_packets = zero_to_inf(value as usize);
            } else if name == "MaxComPacketSize".as_bytes() {
                parsed.max_gross_compacket_size = zero_to_inf(value as usize);
            } else if name == "MaxResponseComPacketSize".as_bytes() {
                parsed.max_gross_compacket_response_size = zero_to_inf(value as usize);
            } else if name == "MaxIndTokenSize".as_bytes() {
                parsed.max_ind_token_size = zero_to_inf(value as usize);
            } else if name == "MaxAggTokenSize".as_bytes() {
                parsed.max_agg_token_size = zero_to_inf(value as usize);
            } else if name == "ContinuedTokens".as_bytes() {
                parsed.continued_tokens = value != 0;
            } else if name == "SequenceNumbers".as_bytes() {
                parsed.seq_numbers = value != 0;
            } else if name == "AckNak".as_bytes() {
                parsed.ack_nak = value != 0;
            } else if name == "Asynchronous".as_bytes() {
                parsed.asynchronous = value != 0;
            } else if name == "DefTransTimeout".as_bytes() {
                parsed.trans_timeout = Duration::from_millis(value as u64)
            };
        }
        parsed
    }

    pub fn common(lhs: &Properties, rhs: &Properties) -> Properties {
        Properties {
            max_methods: std::cmp::min(lhs.max_methods, rhs.max_methods),
            max_subpackets: std::cmp::min(lhs.max_subpackets, rhs.max_subpackets),
            max_gross_packet_size: std::cmp::min(lhs.max_gross_packet_size, rhs.max_gross_packet_size),
            max_packets: std::cmp::min(lhs.max_packets, rhs.max_packets),
            max_gross_compacket_size: std::cmp::min(lhs.max_gross_compacket_size, rhs.max_gross_compacket_size),
            max_gross_compacket_response_size: std::cmp::min(
                lhs.max_gross_compacket_response_size,
                rhs.max_gross_compacket_response_size,
            ),
            max_ind_token_size: std::cmp::min(lhs.max_ind_token_size, rhs.max_ind_token_size),
            max_agg_token_size: std::cmp::min(lhs.max_agg_token_size, rhs.max_agg_token_size),
            continued_tokens: lhs.continued_tokens && rhs.continued_tokens,
            seq_numbers: lhs.seq_numbers && rhs.seq_numbers,
            ack_nak: lhs.ack_nak && rhs.ack_nak,
            asynchronous: lhs.asynchronous && rhs.asynchronous,
            buffer_mgmt: lhs.buffer_mgmt && rhs.buffer_mgmt,
            max_retries: std::cmp::min(lhs.max_retries, rhs.max_retries),
            trans_timeout: std::cmp::min(lhs.trans_timeout, rhs.trans_timeout),
        }
    }
}

fn zero_to_inf(value: usize) -> usize {
    match value {
        0 => usize::MAX,
        _ => value,
    }
}

fn inf_to_zero(value: usize) -> usize {
    match value {
        usize::MAX => 0,
        _ => value,
    }
}

impl Default for Properties {
    fn default() -> Self {
        Properties::ASSUMED
    }
}
