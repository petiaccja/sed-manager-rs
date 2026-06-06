use std::{
    cmp::{max, min},
    collections::VecDeque,
    num::NonZero,
    time::{Duration, Instant},
};

use oneshot::Sender;
use sed_packet::{
    com_id::{COM_ID_PROTOCOL, COM_ID_RESPONSE_LEN, ComIdRequest, ComIdResponse, ComIdResponsePayload},
    packet::{COM_PACKET_HEADER_LEN, ComPacket, PACKET_HEADER_LEN, PACKETIZED_PROTOCOL, SUB_PACKET_HEADER_LEN},
    session_id::SessionId,
};
use sed_spec::methods::{Limit, Properties};
use sorbit::ser_de::{FromBytes, ToBytes};

use crate::{
    Error,
    protocol_sans_io::{
        com_id_session::ComIdSession, rpc_session::RpcSession, sequence_number::SequenceNumber, utility::min_deadline,
    },
};

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

pub struct Protocol {
    com_id: u16,
    com_id_ext: u16,
    rpc_session: RpcSession,
    com_id_session: ComIdSession,
    capabilities: Properties,
    com_packets_sending: VecDeque<ComPacketSendingRecord>,
    com_packet_phase: IfacePhase,
    com_request_phase: IfacePhase,
}

impl Protocol {
    pub fn new(com_id: u16, com_id_ext: u16) -> Self {
        Self {
            com_id,
            com_id_ext,
            rpc_session: RpcSession::new(DEF_TRANS_TIMEOUT, CAPABILITIES),
            com_id_session: ComIdSession::new(DEF_TRANS_TIMEOUT),
            capabilities: CAPABILITIES,
            com_packets_sending: VecDeque::new(),
            com_packet_phase: IfacePhase::Sending,
            com_request_phase: IfacePhase::Sending,
        }
    }

    pub fn handle_method_call(&mut self, session_id: SessionId, call: Vec<u8>, sender: Sender<Result<Vec<u8>, Error>>) {
        self.rpc_session.handle_method_call(session_id, call, sender);
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
        self.com_id_session.notify_time(time);
        self.rpc_session.notify_time(time);

        let deadline = match self.poll_action_com_request(time) {
            Action::None => None,
            action @ Action::Send { .. } => return action,
            action @ Action::Recv { .. } => return action,
            Action::Sleep { until } => Some(until),
        };

        let deadline = match self.poll_action_com_packet(time) {
            Action::None => deadline,
            action @ Action::Send { .. } => return action,
            action @ Action::Recv { .. } => return action,
            Action::Sleep { until } => min_deadline(deadline, Some(until)),
        };

        let deadline =
            min_deadline(deadline, min_deadline(self.com_id_session.poll_timeout(), self.rpc_session.poll_timeout()));

        match deadline {
            Some(until) => Action::Sleep { until },
            None => Action::None,
        }
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
                self.com_request_phase.update_com_request(&response, time);
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
                self.com_packet_phase.update_com_packet(&com_packet, time);
                for packet in com_packet.payload {
                    self.rpc_session.handle_packet(packet);
                }
            }
            Err(_) => (), // Discard received blob.
        }
    }

    fn poll_action_com_packet(&mut self, time: Instant) -> Action {
        let action = poll_phase_action(
            PACKETIZED_PROTOCOL,
            &self.com_packet_phase,
            time,
            self.capabilities.max_gross_compacket_size.get() as usize,
        );
        if matches!(action, Action::None) {
            let packets = self.rpc_session.poll_packets();
            if !packets.is_empty() {
                self.com_packets_sending.push_back(ComPacketSendingRecord {
                    packets: packets
                        .iter()
                        .map(|packet| (SessionId::of(packet), SequenceNumber(packet.sequence_number)))
                        .collect(),
                });
                self.com_packet_phase.update_com_packed_sent(time);

                let com_packet = ComPacket {
                    com_id: self.com_id,
                    com_id_ext: self.com_id_ext,
                    payload: packets,
                    ..Default::default()
                };

                Action::Send {
                    protocol: PACKETIZED_PROTOCOL,
                    data: com_packet.to_bytes().expect("can not serialize ComPacket"),
                }
            } else {
                action
            }
        } else {
            action
        }
    }

    fn poll_action_com_request(&mut self, time: Instant) -> Action {
        let action = poll_phase_action(COM_ID_PROTOCOL, &self.com_request_phase, time, COM_ID_RESPONSE_LEN);
        if matches!(action, Action::None)
            && let Some(request) = self.com_id_session.poll_requests()
        {
            self.com_request_phase.update_com_request_sent(time);
            Action::Send {
                protocol: COM_ID_PROTOCOL,
                data: request.to_bytes().expect("can not serialize ComID request"),
            }
        } else {
            action
        }
    }
}

fn poll_phase_action(protocol: u8, phase: &IfacePhase, time: Instant, max_len: usize) -> Action {
    match phase {
        IfacePhase::Sending => Action::None,
        IfacePhase::Receiving { outstanding_data, min_transfer, attempt, last } => {
            let pause = min(Duration::from_secs(1), Duration::from_millis(1) * (1 << attempt));
            let until = last.clone() + pause;
            if time <= until {
                Action::Sleep { until }
            } else {
                let len = min(max_len, max(*outstanding_data, *min_transfer));
                Action::Recv { protocol, len }
            }
        }
    }
}

struct ComPacketSendingRecord {
    packets: Vec<(SessionId, SequenceNumber)>,
}

pub enum Action {
    None,
    Send { protocol: u8, data: Vec<u8> },
    Recv { protocol: u8, len: usize },
    Sleep { until: Instant },
}

/// Implements the (send-recv-recv-recv...)*n logic of the TCG protocol.
///
/// The send and receive commands must alternate, so two sends following each
/// other is a nono. Multiple receives may be necessary to retrieve the response
/// to the previous send.
enum IfacePhase {
    Sending,
    Receiving { outstanding_data: usize, min_transfer: usize, attempt: u64, last: Instant },
}

impl IfacePhase {
    pub fn update_com_packed_sent(&mut self, time: Instant) {
        *self = Self::Receiving { outstanding_data: 1, min_transfer: 512, attempt: 0, last: time };
    }

    pub fn update_com_request_sent(&mut self, time: Instant) {
        *self = Self::Receiving {
            outstanding_data: COM_ID_RESPONSE_LEN,
            min_transfer: COM_ID_RESPONSE_LEN,
            attempt: 0,
            last: time,
        };
    }

    pub fn update_com_packet(&mut self, com_packet: &ComPacket, time: Instant) {
        let attempt = match self {
            IfacePhase::Sending => 0,
            IfacePhase::Receiving { attempt, .. } => *attempt,
        };
        let has_data = !com_packet.payload.is_empty();
        let updated = match com_packet.outstanding_data {
            0 => Self::Sending,
            _ => Self::Receiving {
                outstanding_data: com_packet.outstanding_data as usize,
                min_transfer: com_packet.min_transfer as usize,
                attempt: if has_data { 0 } else { attempt + 1 },
                last: time,
            },
        };
        *self = updated;
    }

    pub fn update_com_request(&mut self, response: &ComIdResponse, time: Instant) {
        let attempt = match self {
            IfacePhase::Sending => 0,
            IfacePhase::Receiving { attempt, .. } => *attempt,
        };
        let updated = match response.payload {
            ComIdResponsePayload::NoResponseAvailable { .. } => Self::Sending,
            ComIdResponsePayload::Verify { .. } => Self::Sending,
            ComIdResponsePayload::StackReset { available_data_length, .. } => {
                if available_data_length >= 0x04 {
                    Self::Sending
                } else {
                    Self::Receiving {
                        outstanding_data: COM_ID_RESPONSE_LEN,
                        min_transfer: COM_ID_RESPONSE_LEN,
                        attempt: attempt + 1,
                        last: time,
                    }
                }
            }
        };
        *self = updated
    }
}
