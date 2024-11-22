use std::time::{Duration, Instant};

use tokio::select;

use crate::device::Device;
use crate::messaging::packet::{ComPacket, HandleComIdResponseHeader, HANDLE_COM_ID_RESPONSE_LEN};
use crate::rpc::error::Error;
use crate::rpc::pipeline::{BufferedSend, Process, PullInput, PushOutput, Receive};
use crate::rpc::properties::Properties;
use crate::serialization::{Deserialize, InputStream, OutputStream, Serialize};

use super::utils::clamp;

const PROTOCOL_HANDLE_COM_ID: u8 = 0x02;
const PROTOCOL_COM_PACKET: u8 = 0x01;

enum SyncState {
    Send,
    ReceiveHandleComId { com_id: u16 },
    ReceiveComPacket { com_id: u16 },
}

pub struct HandleComIdPacket {
    com_id: u16,
    payload: Vec<u8>,
}

pub struct InterfaceLayer {
    pub handle_com_id_in: PullInput<HandleComIdPacket>,
    pub handle_com_id_out: PushOutput<HandleComIdPacket>,
    pub com_packet_in: PullInput<ComPacket>,
    pub com_packet_out: PushOutput<ComPacket>,
    device: Box<dyn Device + Sync + Send>,
    properties: Properties,
    sync_state: SyncState,
}

impl InterfaceLayer {
    pub fn new(device: Box<dyn Device + Sync + Send>, properties: Properties) -> Self {
        Self {
            handle_com_id_in: PullInput::new(),
            handle_com_id_out: PushOutput::new(),
            com_packet_in: PullInput::new(),
            com_packet_out: PushOutput::new(),
            device: device,
            properties: properties,
            sync_state: SyncState::Send,
        }
    }

    /// Follow the TPer asynchronous protocol.
    async fn update_async(&mut self) -> Result<Option<()>, Error> {
        todo!()
    }

    /// Follow the TPer synchronous protocol.
    async fn update_sync(&mut self) -> Result<Option<()>, Error> {
        match self.sync_state {
            SyncState::Send => self.sync_send().await,
            SyncState::ReceiveHandleComId { com_id } => self.sync_receive_handle_com_id(com_id).await,
            SyncState::ReceiveComPacket { com_id } => self.sync_receive_com_packet(com_id).await,
        }
    }

    async fn sync_send(&mut self) -> Result<Option<()>, Error> {
        select! {
            biased;
            Some(handle_com_id_packet) = self.handle_com_id_in.recv() => {
                let result = self.device.security_send(
                    PROTOCOL_HANDLE_COM_ID,
                    handle_com_id_packet.com_id.to_be_bytes(),
                    handle_com_id_packet.payload.as_slice()
                );
                if let Err(err) = result {
                    Err(Error::InterfaceSendFailed(err))
                }
                else {
                    self.sync_state = SyncState::ReceiveHandleComId{com_id: handle_com_id_packet.com_id};
                    Ok(None)
                }
            },
            Some(com_packet) = self.com_packet_in.recv() => {
                let mut stream = OutputStream::<u8>::new();
                if let Err(err) = com_packet.serialize(&mut stream) {
                    Err(Error::SerializationFailed(err))
                }
                else {let result = self.device.security_send(PROTOCOL_COM_PACKET, com_packet.com_id.to_be_bytes(), stream.as_slice());
                    if let Err(err) = result {
                        Err(Error::InterfaceSendFailed(err))
                    }
                    else {
                        self.sync_state = SyncState::ReceiveComPacket { com_id: com_packet.com_id};
                        Ok(None)
                    }
                }
            },
            else => {
                // For the synchronous protocol, if we are in the sending state and there is nothing to send,
                // we will certainly not receive anything in the future.
                // Consequently, we can terminate this task by returning the result.
                Ok(Some(()))
            },
        }
    }

    /// Get the HANDLE_COM_ID response if the data is not "no response available".
    fn try_recv_handle_com_id(&self, com_id: u16) -> Result<Option<Vec<u8>>, Error> {
        let result =
            self.device.security_recv(PROTOCOL_HANDLE_COM_ID, com_id.to_be_bytes(), HANDLE_COM_ID_RESPONSE_LEN);
        let data = match result {
            Ok(data) => data,
            Err(err) => return Err(Error::InterfaceReceiveFailed(err)),
        };
        let mut stream = InputStream::from(data);
        match HandleComIdResponseHeader::deserialize(&mut stream) {
            Err(err) => Err(Error::SerializationFailed(err)),
            Ok(header) => {
                if header.available_data_len != 0 {
                    Ok(Some(stream.take()))
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Get the ComPacket response if the data is not an empty ComPacket.
    fn try_recv_com_packet(&self, com_id: u16, transfer_len: usize) -> Result<ComPacket, Error> {
        let result = self.device.security_recv(PROTOCOL_COM_PACKET, com_id.to_be_bytes(), transfer_len);
        let data = match result {
            Ok(data) => data,
            Err(err) => return Err(Error::InterfaceReceiveFailed(err)),
        };
        let mut stream = InputStream::from(data);
        match ComPacket::deserialize(&mut stream) {
            Err(err) => Err(Error::SerializationFailed(err)),
            Ok(com_packet) => Ok(com_packet),
        }
    }

    async fn sync_receive_handle_com_id(&mut self, com_id: u16) -> Result<Option<()>, Error> {
        let mut retry_policy = RetryPolicy::new(self.properties.timeout);
        let result = loop {
            match self.try_recv_handle_com_id(com_id) {
                Ok(Some(data)) => {
                    let _ = self.handle_com_id_out.send(HandleComIdPacket { com_id, payload: data });
                    break Ok(None);
                }
                Ok(None) => {
                    if let Err(err) = retry_policy.backoff().await {
                        break Err(err);
                    }
                }
                Err(err) => break Err(err),
            };
        };
        self.sync_state = SyncState::Send;
        result
    }

    async fn sync_receive_com_packet(&mut self, com_id: u16) -> Result<Option<()>, Error> {
        let mut retry_policy = RetryPolicy::new(self.properties.timeout);
        let mut transfer_len = ideal_transfer_len(1024, 1, self.properties.max_gross_compacket_size);
        let result = loop {
            match self.try_recv_com_packet(com_id, transfer_len) {
                Ok(com_packet) => {
                    let has_payload = !com_packet.payload.is_empty();
                    let outstanding_data = com_packet.outstanding_data;
                    transfer_len = ideal_transfer_len(
                        com_packet.outstanding_data as usize,
                        com_packet.min_transfer as usize,
                        self.properties.max_gross_compacket_size,
                    );
                    if has_payload {
                        let _ = self.com_packet_out.send(com_packet);
                    }
                    if outstanding_data == 1 {
                        if let Err(err) = retry_policy.backoff().await {
                            break Err(err);
                        };
                    } else if outstanding_data == 0 {
                        break Ok(None);
                    }
                }
                Err(err) => break Err(err),
            };
        };
        self.sync_state = SyncState::Send;
        result
    }
}

impl Process for InterfaceLayer {
    type Output = ();
    type Error = Error;
    async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
        if self.properties.asynchronous {
            self.update_async().await
        } else {
            self.update_sync().await
        }
    }
}

struct RetryPolicy {
    start_time: Instant,
    deadline: Instant,
    sleep_duration: Duration,
}

impl RetryPolicy {
    fn new(timeout: Duration) -> Self {
        let start_time = Instant::now();
        let deadline = start_time + timeout * 2;
        let sleep_duration = std::cmp::min(timeout / 200, Duration::from_micros(10));
        Self { start_time, deadline, sleep_duration }
    }

    async fn backoff(&mut self) -> Result<(), Error> {
        let current_time = Instant::now();
        if self.deadline <= current_time {
            Err(Error::TimedOut)
        } else {
            tokio::time::sleep(self.sleep_duration).await;
            self.sleep_duration = std::cmp::min(self.sleep_duration, (self.deadline - self.start_time) / 7);
            Ok(())
        }
    }
}

fn ideal_transfer_len(outstanding_data: usize, min_transfer: usize, max_gross_compacket_size: usize) -> usize {
    clamp(outstanding_data, min_transfer, max_gross_compacket_size)
}
