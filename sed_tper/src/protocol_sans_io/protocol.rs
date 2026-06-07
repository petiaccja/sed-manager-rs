use std::{
    collections::VecDeque,
    num::NonZero,
    time::{Duration, Instant},
};

use oneshot::Sender;
use sed_packet::{
    com_id::{COM_ID_PROTOCOL, COM_ID_RESPONSE_LEN, ComIdRequest, ComIdResponse},
    packet::{COM_PACKET_HEADER_LEN, ComPacket, PACKET_HEADER_LEN, PACKETIZED_PROTOCOL, SUB_PACKET_HEADER_LEN},
    session_id::SessionId,
};
use sed_spec::methods::{Limit, Properties};
use sorbit::ser_de::FromBytes;

use crate::{
    Error,
    protocol_sans_io::{
        com_id_session::ComIdSession,
        rpc_session::RpcSession,
        sequence_number::SequenceNumber,
        shared::{Action, min_deadline},
        synchronous_protocol::SynchronousProtocol,
    },
};

use super::{com_id_session::ComIdAction, rpc_session::RpcAction};

/// After the timeout, message sent between the host and device are considered
/// lost.
///
/// When ACK/NAK is not used, the protocol stack will simply use this value to
/// report a timeout in case the message still hasn't been received. This value
/// though is not communicated to the remote. I'm not really sure what's agreed
/// as a timeout in case of no ACK/NAK.
///
/// When ACK/NAK is used, messages need to be ACK-ed within the timeout. The
/// value of the timeout can be communicated in the `StartSession` method.
const DEF_TRANS_TIMEOUT: Duration = Duration::from_secs(if cfg!(feature = "test-utils") { 1 } else { 15 });

const MAX_GROSS_COM_PACKET_SIZE: usize = 1048576;

/// The capabilities supported by the protocol stack implementation.
///
/// Due to the complexities and ambiguities in the specification, asynchronous
/// communication, buffer management, ACK/NAK, and sequence numbers aren't
/// currently implemented. Additionally, most devices don't seem to support
/// these capabilities anyway.
///
/// The maximum packet sizes are generous as the PCs running this software have
/// plenty of RAM. A sensible limit is still necessary to prevent OOM in case
/// the device sends insane amounts of data due to a bug.
pub const CAPABILITIES: Properties = Properties {
    max_methods: Limit::Unlimited,
    max_subpackets: Limit::Unlimited,
    max_gross_packet_size: Limit::Limited(NonZero::new(MAX_GROSS_COM_PACKET_SIZE - COM_PACKET_HEADER_LEN).unwrap()),
    max_packets: Limit::Unlimited,
    max_gross_compacket_size: Limit::Limited(NonZero::new(MAX_GROSS_COM_PACKET_SIZE).unwrap()),
    max_gross_compacket_response_size: Limit::Limited(NonZero::new(MAX_GROSS_COM_PACKET_SIZE).unwrap()),
    max_sessions: None,
    max_read_sessions: None,
    max_ind_token_size: Limit::Limited(
        NonZero::new(MAX_GROSS_COM_PACKET_SIZE - COM_PACKET_HEADER_LEN - PACKET_HEADER_LEN - SUB_PACKET_HEADER_LEN)
            .unwrap(),
    ),
    max_agg_token_size: Limit::Limited(
        NonZero::new(MAX_GROSS_COM_PACKET_SIZE - COM_PACKET_HEADER_LEN - PACKET_HEADER_LEN - SUB_PACKET_HEADER_LEN)
            .unwrap(),
    ),
    max_authentications: None,
    max_transaction_limit: None,
    def_session_timeout: None,
    max_session_timeout: None,
    min_session_timeout: None,
    def_trans_timeout: None,
    max_trans_timeout: None,
    min_trans_timeout: None,
    max_com_id_time: None,
    continued_tokens: false,
    seq_numbers: false,
    ack_nak: false,
    asynchronous: false,
};

#[derive(Debug)]
pub struct Protocol {
    com_id: u16,
    com_id_ext: u16,
    rpc_session: RpcSession,
    com_id_session: ComIdSession,
    rpc_protocol: SynchronousProtocol<ComPacket, ComPacket>,
    com_id_protocol: SynchronousProtocol<ComIdRequest, ComIdResponse>,
    com_packets_sending: VecDeque<ComPacketSendingRecord>,
}

impl Protocol {
    pub fn new(com_id: u16, com_id_ext: u16) -> Self {
        Self {
            com_id,
            com_id_ext,
            rpc_session: RpcSession::new(DEF_TRANS_TIMEOUT, CAPABILITIES),
            com_id_session: ComIdSession::new(DEF_TRANS_TIMEOUT),
            rpc_protocol: SynchronousProtocol::new(PACKETIZED_PROTOCOL, CAPABILITIES.max_gross_compacket_size.get()),
            com_id_protocol: SynchronousProtocol::new(COM_ID_PROTOCOL, COM_ID_RESPONSE_LEN),
            com_packets_sending: VecDeque::new(),
        }
    }

    pub fn handle_method_call(&mut self, session_id: SessionId, call: Vec<u8>, sender: Sender<Result<Vec<u8>, Error>>) {
        self.rpc_session.handle_method_call(session_id, call, sender);
    }

    pub fn handle_session_aborted(&mut self, session_id: SessionId) {
        self.rpc_session.handle_session_aborted(session_id);
    }

    pub fn handle_com_request(&mut self, request: ComIdRequest, sender: Sender<Result<ComIdResponse, Error>>) {
        self.com_id_session.handle_com_request(request, sender);
    }

    pub fn handle_iface_send_done(&mut self, time: Instant, protocol: u8, result: Result<(), Error>) {
        match protocol {
            COM_ID_PROTOCOL => self.handle_iface_com_request_send_done(time, result),
            PACKETIZED_PROTOCOL => self.handle_iface_com_packet_send_done(time, result),
            _ => (),
        }
    }

    pub fn handle_iface_recv_done(&mut self, time: Instant, protocol: u8, result: Result<Vec<u8>, Error>) {
        match protocol {
            COM_ID_PROTOCOL => self.handle_iface_com_request_recv_done(time, result),
            PACKETIZED_PROTOCOL => self.handle_iface_com_packet_recv_done(time, result),
            _ => (),
        }
    }

    pub fn poll_action(&mut self, time: Instant) -> Action {
        let mut deadline = None;

        // Poll the ComID and RPC sessions. At this phase, we're not making any
        // final conclusions about the next action. If they wanna send something,
        // we just queue it in the synchronous protocols.
        match self.com_id_session.poll_action(time) {
            ComIdAction::None => (),
            ComIdAction::Sleep { until } => deadline = min_deadline(deadline, Some(until)),
            ComIdAction::Send(com_id_request) => self.com_id_protocol.handle_send(com_id_request),
        };

        match self.rpc_session.poll_action(time) {
            RpcAction::None => (),
            RpcAction::Sleep { until } => deadline = min_deadline(deadline, Some(until)),
            RpcAction::Send(packets) => {
                self.com_packets_sending.push_back(ComPacketSendingRecord {
                    packets: packets
                        .iter()
                        .map(|packet| (SessionId::of(packet), SequenceNumber(packet.sequence_number)))
                        .collect(),
                });
                self.rpc_protocol.handle_send(ComPacket {
                    com_id: self.com_id,
                    com_id_ext: self.com_id_ext,
                    payload: packets,
                    ..Default::default()
                })
            }
        };

        // Poll the ComID and RPC protocol. In case they want to send or receive,
        // the action is immediately returned. The two protocols are eventually
        // independent, so a send-receive cycle on the ComID protocol might
        // overlap with a send-receive cycle on the RPC protocol, and that's
        // still not a violation of the synchronous protocol.
        //
        // Since the ComID protocol (0x02) is polled first, it gets a precedence.
        // This is important because this way an IF-SEND for stack reset can be
        // issued before all IF-RECVs complete for the token stream protocol.
        // This can be used to interrupt pending RPCs on protocol 0x01.
        match self.com_id_protocol.poll_action(time) {
            Action::None => (),
            Action::Sleep { until } => deadline = min_deadline(deadline, Some(until)),
            action @ Action::Send { .. } => return action,
            action @ Action::Recv { .. } => return action,
        }

        match self.rpc_protocol.poll_action(time) {
            Action::None => (),
            Action::Sleep { until } => deadline = min_deadline(deadline, Some(until)),
            action @ Action::Send { .. } => return action,
            action @ Action::Recv { .. } => return action,
        }

        deadline.map(|deadline| Action::Sleep { until: deadline }).unwrap_or(Action::None)
    }

    fn handle_iface_com_packet_send_done(&mut self, time: Instant, result: Result<(), Error>) {
        if let Some(record) = self.com_packets_sending.pop_front() {
            for (session_id, sn) in record.packets {
                self.rpc_session.handle_iface_send_done(time, session_id, sn, result.clone());
            }
        }
    }

    fn handle_iface_com_request_send_done(&mut self, time: Instant, result: Result<(), Error>) {
        self.com_id_session.handle_iface_send_done(time, result);
    }

    fn handle_iface_com_request_recv_done(&mut self, time: Instant, result: Result<Vec<u8>, Error>) {
        let Ok(data) = result else {
            return;
        };
        match ComIdResponse::from_bytes(&data) {
            Ok(response) => {
                self.com_id_protocol.handle_recv(time, &response);
                self.com_id_session.handle_iface_recv_done(response);
            }
            Err(_) => (), // Discard received blob.
        }
    }

    fn handle_iface_com_packet_recv_done(&mut self, time: Instant, result: Result<Vec<u8>, Error>) {
        let Ok(data) = result else {
            return;
        };
        match ComPacket::from_bytes(&data) {
            Ok(com_packet) => {
                self.rpc_protocol.handle_recv(time, &com_packet);
                for packet in com_packet.payload {
                    self.rpc_session.handle_packet(packet);
                }
            }
            Err(_) => (), // Discard received blob.
        }
    }
}

#[derive(Debug)]
struct ComPacketSendingRecord {
    packets: Vec<(SessionId, SequenceNumber)>,
}

#[cfg(test)]
mod tests {
    use googletest::assert_that;
    use googletest::matchers::*;
    use oneshot::channel;
    use sed_packet::com_id::ComIdResponsePayload;
    use sed_packet::com_id::StackResetStatus;
    use sed_spec::methods::MethodStatus;
    use sorbit::ser_de::ToBytes;

    use super::*;

    use crate::protocol_sans_io::shared::packetize_one;
    use crate::protocol_sans_io::shared::tests::*;

    const SESSION_ID: SessionId = SessionId { hsn: 1, tsn: 2 };

    #[test]
    fn com_id_request_completed() {
        let mut protocol = Protocol::new(1, 0);
        let (sender, receiver) = channel();

        let request = ComIdRequest::stack_reset(1, 0);
        let response = ComIdResponse {
            com_id: 1,
            com_id_ext: 0,
            payload: ComIdResponsePayload::StackReset { available_data_length: 4, status: StackResetStatus::Success },
        };

        // Issue request.
        protocol.handle_com_request(request.clone(), sender);

        // "Send" call to device.
        let action = protocol.poll_action(Instant::now());
        assert_that!(action, eq(&Action::Send { protocol: 0x02, data: request.to_bytes().unwrap() }));
        protocol.handle_iface_send_done(Instant::now(), 0x02, Ok(()));

        // "Recv" response from device.
        let action = protocol.poll_action(Instant::now());
        assert_that!(action, eq(&Action::Recv { protocol: 0x02, transfer_len: 46 }));
        protocol.handle_iface_recv_done(Instant::now(), 0x02, Ok(response.to_bytes().unwrap()));

        // Poll to completion.
        let action = protocol.poll_action(Instant::now());
        assert_that!(action, eq(&Action::None));

        // Check if we received the response to the method call.
        assert_that!(receiver.try_recv(), ok(ok(eq(&response))));
    }

    #[test]
    fn method_call_completed() {
        let mut protocol = Protocol::new(1, 0);
        let (sender, receiver) = channel();

        let call = start_session_call(SESSION_ID);
        let response = sync_session_call(SESSION_ID, MethodStatus::Success);
        let call_com_packet = ComPacket {
            com_id: 1,
            com_id_ext: 0,
            outstanding_data: 0,
            min_transfer: 0,
            length: std::marker::PhantomData,
            payload: vec![packetize_one(
                SessionId::MANAGEMENT,
                SequenceNumber(1),
                call.clone(),
            )],
        };
        let response_com_packet = ComPacket {
            com_id: 1,
            com_id_ext: 0,
            outstanding_data: 0,
            min_transfer: 0,
            length: std::marker::PhantomData,
            payload: vec![packetize_one(
                SessionId::MANAGEMENT,
                SequenceNumber(1),
                response.clone(),
            )],
        };

        // Issue method call.
        protocol.handle_method_call(SessionId::MANAGEMENT, call, sender);

        // "Send" call to device.
        let action = protocol.poll_action(Instant::now());
        assert_that!(action, eq(&Action::Send { protocol: 0x01, data: call_com_packet.to_bytes().unwrap() }));
        protocol.handle_iface_send_done(Instant::now(), 0x01, Ok(()));

        // "Recv" response from device.
        let action = protocol.poll_action(Instant::now());
        assert_that!(action, eq(&Action::Recv { protocol: 0x01, transfer_len: 512 }));
        protocol.handle_iface_recv_done(Instant::now(), 0x01, Ok(response_com_packet.to_bytes().unwrap()));

        // Poll to completion.
        let action = protocol.poll_action(Instant::now());
        assert_that!(action, eq(&Action::None));

        // Check if we received the response to the method call.
        assert_that!(receiver.try_recv(), ok(ok(eq(&response))));
    }
}
