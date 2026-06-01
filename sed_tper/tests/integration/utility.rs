use std::time::Duration;

use sorbit::ser_de::ToBytes;

use sed_device::Error as DeviceError;
use sed_device::mock_device::MockEvent;
use sed_packet::packet::{ComPacket, Packet, SubPacket, SubPacketKind};
use sed_packet::token::ToTokens;

pub const TEST_TIMEOUT: Duration = Duration::from_secs(5);

pub fn method_call_event(com_id: u16, hsn: u32, tsn: u32, call: impl ToTokens) -> MockEvent {
    MockEvent::Send {
        name: Some("method_call".into()),
        security_protocol: 0x01,
        protocol_specific: com_id.to_be_bytes(),
        expected: packetize(com_id, hsn, tsn, call).to_bytes().unwrap(),
        result: Ok(()),
    }
}

pub fn method_return_event(com_id: u16, hsn: u32, tsn: u32, return_: impl ToTokens) -> MockEvent {
    MockEvent::Recv {
        name: Some("method_return".into()),
        security_protocol: 0x01,
        protocol_specific: com_id.to_be_bytes(),
        result: Ok(packetize(com_id, hsn, tsn, return_).to_bytes().unwrap()),
    }
}

pub fn method_call_fail_event(com_id: u16, hsn: u32, tsn: u32, call: impl ToTokens, error: DeviceError) -> MockEvent {
    MockEvent::Send {
        name: Some("method_call".into()),
        security_protocol: 0x01,
        protocol_specific: com_id.to_be_bytes(),
        expected: packetize(com_id, hsn, tsn, call).to_bytes().unwrap(),
        result: Err(error),
    }
}

pub fn method_return_fail_event(com_id: u16, error: DeviceError) -> MockEvent {
    MockEvent::Recv {
        name: Some("method_return".into()),
        security_protocol: 0x01,
        protocol_specific: com_id.to_be_bytes(),
        result: Err(error),
    }
}

pub fn com_id_request_event<const MULTI_PASS: bool>(com_id: u16, request: impl ToBytes<MULTI_PASS>) -> MockEvent {
    MockEvent::Send {
        name: Some("com_id_request".into()),
        security_protocol: 0x02,
        protocol_specific: com_id.to_be_bytes(),
        expected: request.to_bytes().unwrap(),
        result: Ok(()),
    }
}

pub fn com_id_response_event<const MULTI_PASS: bool>(com_id: u16, response: impl ToBytes<MULTI_PASS>) -> MockEvent {
    MockEvent::Recv {
        name: Some("com_id_response".into()),
        security_protocol: 0x02,
        protocol_specific: com_id.to_be_bytes(),
        result: Ok(response.to_bytes().unwrap()),
    }
}

pub fn com_id_request_fail_event<const MULTI_PASS: bool>(
    com_id: u16,
    request: impl ToBytes<MULTI_PASS>,
    error: DeviceError,
) -> MockEvent {
    MockEvent::Send {
        name: Some("com_id_request".into()),
        security_protocol: 0x02,
        protocol_specific: com_id.to_be_bytes(),
        expected: request.to_bytes().unwrap(),
        result: Err(error),
    }
}

pub fn com_id_response_fail_event(com_id: u16, error: DeviceError) -> MockEvent {
    MockEvent::Recv {
        name: Some("com_id_response".into()),
        security_protocol: 0x02,
        protocol_specific: com_id.to_be_bytes(),
        result: Err(error),
    }
}

fn packetize(com_id: u16, hsn: u32, tsn: u32, value: impl ToTokens) -> ComPacket {
    ComPacket {
        com_id,
        com_id_ext: 0,
        payload: vec![Packet {
            tper_session_number: tsn,
            host_session_number: hsn,
            payload: vec![SubPacket {
                kind: SubPacketKind::Data,
                length: std::marker::PhantomData,
                payload: value.to_tokens().unwrap(),
            }],
            ..Default::default()
        }],
        ..Default::default()
    }
}
