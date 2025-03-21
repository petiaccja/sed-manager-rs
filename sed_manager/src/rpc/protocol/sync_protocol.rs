//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::device::Device;
use crate::messaging::com_id::{
    HandleComIdRequest, HandleComIdResponse, HANDLE_COM_ID_PROTOCOL, HANDLE_COM_ID_RESPONSE_LEN,
};
use crate::messaging::packet::{ComPacket, PACKETIZED_PROTOCOL};
use crate::rpc::{Error, Properties};
use crate::serialization::{DeserializeBinary as _, SerializeBinary};

use super::retry::Retry;

pub async fn roundtrip_packet(
    device: &dyn Device,
    com_id: u16,
    com_packet: ComPacket,
    properties: &Properties,
) -> Result<ComPacket, Error> {
    let protocol = PACKETIZED_PROTOCOL;
    let protocol_specific = com_id.to_be_bytes();

    let req_bytes = com_packet.to_bytes()?;
    device.security_send(protocol, protocol_specific, &req_bytes)?;

    let mut retry = Retry::new(properties.trans_timeout);
    let mut com_packet = ComPacket::default();
    loop {
        let transfer_len = optimal_transfer_len(properties, com_packet.min_transfer, com_packet.outstanding_data);
        match recv_partial_packet(device, com_id, transfer_len) {
            Ok(new_com_packet) => com_packet.append(new_com_packet),
            Err(error) => break Err(error),
        }
        if com_packet.outstanding_data != 0 {
            retry.sleep().await?;
        } else {
            break Ok(com_packet);
        }
    }
}

pub async fn roundtrip_com_id(
    device: &dyn Device,
    com_id: u16,
    request: HandleComIdRequest,
    properties: &Properties,
) -> Result<HandleComIdResponse, Error> {
    let protocol = HANDLE_COM_ID_PROTOCOL;
    let protocol_specific = com_id.to_be_bytes();

    let req_bytes = request.to_bytes()?;
    device.security_send(protocol, protocol_specific, &req_bytes)?;

    let mut retry = Retry::new(properties.trans_timeout);
    loop {
        let transfer_len = HANDLE_COM_ID_RESPONSE_LEN;
        let protocol_specific = com_id.to_be_bytes();
        let data = device.security_recv(protocol, protocol_specific, transfer_len)?;
        let response = HandleComIdResponse::from_bytes(data)?;
        if response.payload.is_empty() {
            retry.sleep().await?;
        } else {
            break Ok(response);
        }
    }
}

fn optimal_transfer_len(properties: &Properties, min_transfer: u32, outstanding_data: u32) -> usize {
    let limit = properties.max_gross_compacket_response_size;
    let desired = core::cmp::max(512, core::cmp::min(limit, outstanding_data as usize));
    core::cmp::max(min_transfer as usize, desired)
}

fn recv_partial_packet(device: &dyn Device, com_id: u16, transfer_len: usize) -> Result<ComPacket, Error> {
    let protocol_specific = com_id.to_be_bytes();
    let data = device.security_recv(PACKETIZED_PROTOCOL, protocol_specific, transfer_len)?;
    Ok(ComPacket::from_bytes(data)?)
}
