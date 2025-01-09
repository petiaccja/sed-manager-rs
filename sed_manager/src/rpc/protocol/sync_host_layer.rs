use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::{mpsc, Mutex};

use super::interface_layer::InterfaceLayer;
use super::retry::Retry;
use crate::device::Device;
use crate::messaging::com_id::{
    HandleComIdRequest, HandleComIdResponse, HANDLE_COM_ID_PROTOCOL, HANDLE_COM_ID_RESPONSE_LEN,
};
use crate::messaging::packet::{ComPacket, PACKETIZED_PROTOCOL};
use crate::rpc::error::Error;
use crate::rpc::properties::Properties;
use crate::serialization::{Deserialize, InputStream, OutputStream, Serialize};

pub struct SyncHostLayer {
    device: Mutex<Arc<dyn Device>>,
    com_id: u16,
    properties: Properties,
    tx_handle_com_id: Mutex<mpsc::UnboundedSender<Result<HandleComIdResponse, Error>>>,
    rx_handle_com_id: Mutex<mpsc::UnboundedReceiver<Result<HandleComIdResponse, Error>>>,
    tx_com_packet: Mutex<mpsc::UnboundedSender<Result<ComPacket, Error>>>,
    rx_com_packet: Mutex<mpsc::UnboundedReceiver<Result<ComPacket, Error>>>,
}

impl SyncHostLayer {
    pub fn new(device: Arc<dyn Device>, com_id: u16, properties: Properties) -> Self {
        let (tx_handle_com_id, rx_handle_com_id) = mpsc::unbounded_channel();
        let (tx_com_packet, rx_com_packet) = mpsc::unbounded_channel();
        Self {
            device: device.into(),
            com_id,
            properties,
            tx_handle_com_id: tx_handle_com_id.into(),
            rx_handle_com_id: rx_handle_com_id.into(),
            tx_com_packet: tx_com_packet.into(),
            rx_com_packet: rx_com_packet.into(),
        }
    }
}

#[async_trait]
impl InterfaceLayer for SyncHostLayer {
    async fn send_handle_com_id(&self, request: HandleComIdRequest) -> Result<(), Error> {
        let device = self.device.lock().await;
        security_send_handle_com_id(device.deref().deref(), self.com_id, request)?;

        let result = security_recv_handle_com_id(device.deref().deref(), self.com_id, self.properties.timeout).await;
        let _ = self.tx_handle_com_id.lock().await.send(result);
        Ok(())
    }

    async fn recv_handle_com_id(&self) -> Result<HandleComIdResponse, Error> {
        if let Some(result) = self.rx_handle_com_id.lock().await.recv().await {
            result
        } else {
            Err(Error::Closed)
        }
    }

    async fn send_com_packet(&self, request: ComPacket) -> Result<(), Error> {
        let device = self.device.lock().await;
        security_send_com_packet(device.deref().deref(), self.com_id, request)?;

        let result = security_recv_com_packet(
            device.deref().deref(),
            self.com_id,
            self.properties.timeout,
            self.properties.max_gross_compacket_size as u32,
        )
        .await;
        let _ = self.tx_com_packet.lock().await.send(result);
        Ok(())
    }

    async fn recv_com_packet(&self) -> Result<ComPacket, Error> {
        if let Some(result) = self.rx_com_packet.lock().await.recv().await {
            result
        } else {
            Err(Error::Closed)
        }
    }

    async fn close(&self) {
        ()
    }
}

fn security_send_handle_com_id(device: &dyn Device, com_id: u16, request: HandleComIdRequest) -> Result<(), Error> {
    let com_id = com_id.to_be_bytes();

    let mut os = OutputStream::<u8>::new();
    if let Err(err) = request.serialize(&mut os) {
        return Err(Error::SerializationFailed(err));
    };
    if let Err(err) = device.security_send(HANDLE_COM_ID_PROTOCOL, com_id, os.take().as_slice()) {
        return Err(Error::SecuritySendFailed(err));
    };
    Ok(())
}

async fn security_recv_handle_com_id(
    device: &dyn Device,
    com_id: u16,
    timeout: Duration,
) -> Result<HandleComIdResponse, Error> {
    let com_id = com_id.to_be_bytes();

    let mut retry = Retry::new(timeout);
    loop {
        let data = match device.security_recv(HANDLE_COM_ID_PROTOCOL, com_id, HANDLE_COM_ID_RESPONSE_LEN) {
            Ok(data) => data,
            Err(err) => return Err(Error::SecurityReceiveFailed(err)),
        };
        let mut is = InputStream::from(data);
        let response = match HandleComIdResponse::deserialize(&mut is) {
            Ok(data) => data,
            Err(err) => return Err(Error::SerializationFailed(err)),
        };
        if !response.payload.is_empty() {
            break Ok(response);
        } else if let Err(err) = retry.sleep().await {
            break Err(err);
        }
    }
}

fn security_send_com_packet(device: &dyn Device, com_id: u16, request: ComPacket) -> Result<(), Error> {
    let com_id = com_id.to_be_bytes();

    let mut os = OutputStream::<u8>::new();
    if let Err(err) = request.serialize(&mut os) {
        return Err(Error::SerializationFailed(err));
    };
    if let Err(err) = device.security_send(PACKETIZED_PROTOCOL, com_id, os.take().as_slice()) {
        return Err(Error::SecuritySendFailed(err));
    };
    Ok(())
}

async fn security_recv_com_packet(
    device: &dyn Device,
    com_id: u16,
    timeout: Duration,
    max_com_packet_size: u32,
) -> Result<ComPacket, Error> {
    let com_id = com_id.to_be_bytes();

    let mut retry = Retry::new(timeout);
    let mut receive_buffer_len: u32 = 512;
    let mut packets = Vec::new();
    loop {
        let data = match device.security_recv(PACKETIZED_PROTOCOL, com_id, receive_buffer_len as usize) {
            Ok(data) => data,
            Err(err) => return Err(Error::SecurityReceiveFailed(err)),
        };
        let mut is = InputStream::from(data);
        let response = match ComPacket::deserialize(&mut is) {
            Ok(data) => data,
            Err(err) => return Err(Error::SerializationFailed(err)),
        };
        for packet in response.payload.into_vec() {
            packets.push(packet);
        }
        receive_buffer_len =
            std::cmp::max(response.outstanding_data, std::cmp::min(max_com_packet_size, response.min_transfer));
        if response.outstanding_data == 0 {
            break Ok(ComPacket { payload: packets.into(), ..response });
        } else if let Err(err) = retry.sleep().await {
            break Err(err);
        };
    }
}
