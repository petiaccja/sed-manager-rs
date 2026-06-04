use core::mem::replace;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::time::{Duration, Instant};

use sed_async::cancel_channel;
use sed_packet::packet::{PACKET_HEADER_LEN, Packet, SUB_PACKET_HEADER_LEN, SubPacket, SubPacketKind};
use sed_packet::session_id::SessionId;
use sed_packet::token::{Command, ToTokens as _};
use sed_spec::methods::{ExtractResult, Properties, extract_method};

use tracing::field::debug;
use tracing::{Span, instrument};

use crate::error::Error;
use crate::protocol::message::{
    CommitBatch, Delete, Message, MethodResponse, PacketReceived, ReportAborted, SendMethod, SendPacket, SendPacketDone,
};
use crate::protocol::method::{AnyMethodResult, RecvQueuedMethod, WriteQueuedMethod, retain_alive};
use crate::protocol::protocol::{Address, Context};

#[derive(Debug)]
pub struct Session {
    session_id: SessionId,
    timeout: Duration,
    properties: Properties,
    state: State,
}

impl Session {
    pub fn new(session_id: SessionId, properties: Properties, timeout: Duration) -> Self {
        Self {
            session_id,
            timeout,
            properties,
            state: State::Active {
                send_method_queue: VecDeque::new(),
                receive_buffer: VecDeque::new(),
                channel_queue: VecDeque::new(),
            },
        }
    }

    fn address(&self) -> Address {
        Address::Session(self.session_id)
    }

    #[instrument(level = "debug", skip_all)]
    pub fn send_method(&mut self, context: Context, SendMethod { method, channel }: SendMethod) {
        let address = self.address();
        self.state = match replace(&mut self.state, State::Closed) {
            State::Active { mut send_method_queue, receive_buffer, channel_queue } => {
                let is_end_of_session = is_end_of_session(&method);

                let send_commit_batch = send_method_queue.is_empty();
                send_method_queue.push_back(SendMethod { method, channel });

                if !is_end_of_session {
                    if send_commit_batch {
                        context.send(address, Message::CommitBatch(CommitBatch));
                    }
                    State::Active { send_method_queue, receive_buffer, channel_queue }
                } else {
                    let packets = packetize_method_queue(&self.properties, send_method_queue);
                    send_packets(self.session_id, context, packets);
                    State::Closing { receive_buffer, channel_queue }
                }
            }
            state @ State::Closing { .. } => {
                let _ = channel.send(Err(Error::Closed));
                state
            }
            state @ State::Closed => {
                let _ = channel.send(Err(Error::Closed));
                state
            }
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn commit_batch(&mut self, context: Context, _message: CommitBatch) {
        match &mut self.state {
            State::Active { send_method_queue, .. } => {
                let packets = packetize_method_queue(&self.properties, replace(send_method_queue, VecDeque::new()));
                send_packets(self.session_id, context, packets);
            }
            State::Closing { .. } => (),
            State::Closed => (),
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn send_packet_done(&mut self, context: Context, SendPacketDone { status, methods }: SendPacketDone) {
        let address = self.address();
        match &mut self.state {
            State::Active { channel_queue, .. } | State::Closing { channel_queue, .. } => match &status {
                Ok(_) => {
                    let deadline = Instant::now() + self.timeout;
                    let (cancel_token, cancel_sender) = cancel_channel();
                    context.send_timeout(address.clone(), deadline, Some(cancel_token));
                    for WriteQueuedMethod { channel, .. } in methods {
                        channel_queue.push_back(RecvQueuedMethod {
                            channel,
                            deadline,
                            cancel_sender: None,
                            mgmt_session_meta: None,
                        });
                    }
                    // The cancel sender must be added to the last method in this batch.
                    // Otherwise, the first method could complete, cancel the timeout, and
                    // leave the last method to wait forever.
                    channel_queue.back_mut().map(|recv_queued_method| {
                        recv_queued_method.cancel_sender = Some(cancel_sender);
                    });
                }
                Err(err) => {
                    for WriteQueuedMethod { channel, .. } in methods {
                        let _ = channel.send(Err(err.clone()));
                    }
                }
            },
            State::Closed => {
                for WriteQueuedMethod { channel, .. } in methods {
                    let _ = channel.send(Err(Error::Closed));
                }
            }
        }
    }

    #[instrument(level = "debug", skip(self, context))]
    pub fn timeout(&mut self, context: Context, time: Instant) {
        match &mut self.state {
            State::Active { channel_queue, .. } | State::Closing { channel_queue, .. } => {
                if retain_alive(time, channel_queue) > 0 {
                    self.abort(context);
                }
            }
            State::Closed => (),
        }
    }

    #[instrument(level = "debug", skip_all, fields(error, tokens))]
    pub fn packet_reveived(&mut self, context: Context, PacketReceived { packet }: PacketReceived) {
        match &mut self.state {
            State::Active { receive_buffer, channel_queue, .. }
            | State::Closing { receive_buffer, channel_queue, .. } => {
                assert_eq!(SessionId::of(&packet), self.session_id, "received packet with incorrect HSN/TSN");

                for SubPacket { kind, payload, .. } in packet.payload {
                    if kind == SubPacketKind::Data {
                        receive_buffer.extend(payload);
                    }
                }

                match extract_method::<AnyMethodResult>(receive_buffer) {
                    ExtractResult::Ok { value: _, tokens } => {
                        if let Some(RecvQueuedMethod { channel, cancel_sender, .. }) = channel_queue.pop_front() {
                            cancel_sender.map(|cancel_sender| cancel_sender.cancel());
                            Span::current().record("tokens", debug(&tokens));
                            let _ = channel.send(Ok(tokens));
                        } else {
                            // Either the device sent too much stuff, or there is a packet distribution bug.
                            Span::current()
                                .record("error", "too many methods received from device, or protocol routing bug")
                                .record("tokens", debug(&tokens));
                            self.abort(context);
                        }
                    }
                    ExtractResult::NeedMoreTokens => (),
                    ExtractResult::EndOfStream => {
                        if let Some(RecvQueuedMethod { channel, cancel_sender, .. }) = channel_queue.pop_front() {
                            cancel_sender.map(|cancel_sender| cancel_sender.cancel());
                            let _ = channel.send(Ok(Command::EndOfSession.to_tokens().unwrap()));
                        }
                        self.shutdown(context)
                    }
                    ExtractResult::InvalidTokens(err) => {
                        Span::current()
                            .record("error", tracing::field::debug(&err))
                            .record("tokens", debug(receive_buffer));
                        self.abort(context)
                    }
                };
            }
            State::Closed => (),
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub fn abort(&mut self, context: Context) {
        match &self.state {
            State::Active { .. } => {
                // Send an END_OF_SESSION to the TPer.
                let packet = self.session_id.assign(Packet {
                    payload: vec![SubPacket {
                        kind: SubPacketKind::Data,
                        length: PhantomData,
                        payload: Command::EndOfSession.to_tokens().expect("serializing a command should never fail"),
                    }],
                    ..Default::default()
                });
                context.send(
                    Address::DeviceSession,
                    Message::SendPacket(SendPacket { sender: self.address(), packet, methods: vec![] }),
                );
            }
            _ => (),
        };
        self.shutdown(context);
    }

    #[instrument(level = "debug", skip_all)]
    pub fn report_aborted(&mut self, context: Context, _message: ReportAborted) {
        self.shutdown(context);
    }

    fn shutdown(&mut self, context: Context) {
        self.abort_pending_methods();
        context.send(Address::Control, Message::Delete(Delete(self.session_id)));
    }

    fn abort_pending_methods(&mut self) {
        match replace(&mut self.state, State::Closed) {
            State::Active { send_method_queue, channel_queue, .. } => {
                for SendMethod { channel, .. } in send_method_queue {
                    let _ = channel.send(Err(Error::Aborted));
                }
                for RecvQueuedMethod { channel, .. } in channel_queue {
                    let _ = channel.send(Err(Error::Aborted));
                }
            }
            State::Closing { channel_queue, .. } => {
                for RecvQueuedMethod { channel, .. } in channel_queue {
                    let _ = channel.send(Err(Error::Aborted));
                }
            }
            State::Closed => (),
        };
    }
}

#[derive(Debug)]
enum State {
    Active {
        send_method_queue: VecDeque<SendMethod>,
        receive_buffer: VecDeque<u8>,
        channel_queue: VecDeque<RecvQueuedMethod>,
    },
    Closing {
        receive_buffer: VecDeque<u8>,
        channel_queue: VecDeque<RecvQueuedMethod>,
    },
    Closed,
}

fn is_end_of_session(bytes: &[u8]) -> bool {
    bytes.first() == Command::EndOfSession.to_tokens().expect("tokenization of commands should never fail").first()
}

fn packetize_method_queue(
    properties: &Properties,
    mut send_method_queue: VecDeque<SendMethod>,
) -> Vec<(Packet, oneshot::Sender<MethodResponse>)> {
    let max_method_size =
        std::cmp::max(PACKET_HEADER_LEN + SUB_PACKET_HEADER_LEN, properties.max_gross_packet_size.get())
            - PACKET_HEADER_LEN
            + SUB_PACKET_HEADER_LEN;
    let mut packets = Vec::new();
    while let Some(SendMethod { method, channel }) = send_method_queue.pop_front() {
        if method.len() <= max_method_size {
            let sub_packet = SubPacket { kind: SubPacketKind::Data, length: PhantomData, payload: method };
            let packet = Packet { payload: vec![sub_packet], ..Default::default() };
            packets.push((packet, channel));
        } else {
            let _ = channel.send(Err(Error::MethodTooLarge));
        }
    }
    packets
}

fn send_packets(session_id: SessionId, context: Context, packets: Vec<(Packet, oneshot::Sender<MethodResponse>)>) {
    for (packet, channel) in packets {
        context.send(
            Address::DeviceSession,
            Message::SendPacket(SendPacket {
                sender: session_id.into(),
                packet: session_id.assign(packet),
                methods: vec![WriteQueuedMethod { channel, mgmt_session_meta: None }],
            }),
        );
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::protocol::protocol::DispatchMessage;

    use super::*;

    use googletest::{assert_that, prelude::*};
    use sed_packet::token::ToTokens;
    use sed_spec::methods::{Activate, ActivateResult, MethodCall, MethodResult, MethodStatus};
    use sed_spec::preconfig::core::shared::method_id::ACTIVATE;
    use sed_spec::preconfig::opal_2::admin::sp;

    const TEST_TIMEOUT: Duration = Duration::from_millis(500);

    fn create_request() -> MethodCall<Activate> {
        MethodCall {
            invoking_id: sp::LOCKING.to_uid(),
            method_id: ACTIVATE.to_uid(),
            parameters: Activate,
            status: MethodStatus::Success,
        }
    }

    fn create_response() -> MethodResult<ActivateResult> {
        MethodResult(Ok(ActivateResult))
    }

    #[test]
    fn send_method_regular() {
        let session_id = SessionId { hsn: 1, tsn: 2 };
        let mut session = Session::new(session_id, Properties::ASSUMED, TEST_TIMEOUT);
        let (context, queue) = Context::mock();
        let (tx, rx) = oneshot::channel();
        let method = create_request();
        session.send_method(context, SendMethod { method: method.to_tokens().unwrap(), channel: tx });

        let DispatchMessage { address, message, .. } = queue.try_recv().unwrap();
        assert_that!(address, eq(&Address::Session(session_id)));
        assert_that!(message, matches_pattern!(Message::CommitBatch(_)));
        assert!(queue.is_empty());
        assert_that!(session.state, field!(State::Active.send_method_queue, len(eq(1))));
        assert_that!(rx.has_message(), eq(false));
    }

    #[test]
    fn send_method_eos() {
        let session_id = SessionId { hsn: 1, tsn: 2 };
        let mut session = Session::new(session_id, Properties::ASSUMED, TEST_TIMEOUT);
        let (context, queue) = Context::mock();
        let (tx, rx) = oneshot::channel();
        let method = Command::EndOfSession;
        session.send_method(context, SendMethod { method: method.to_tokens().unwrap(), channel: tx });

        let DispatchMessage { address, message, .. } = queue.try_recv().unwrap();
        assert_that!(address, eq(&Address::DeviceSession));
        assert_that!(message, matches_pattern!(Message::SendPacket(_)));
        assert!(queue.is_empty());
        assert_that!(session.state, matches_pattern!(State::Closing { .. }));
        assert_that!(rx.has_message(), eq(false));
    }

    #[test]
    fn commit_batch() {
        let session_id = SessionId { hsn: 1, tsn: 2 };
        let mut session = Session::new(session_id, Properties::ASSUMED, TEST_TIMEOUT);
        let (context, queue) = Context::mock();
        let (tx, rx) = oneshot::channel();
        let method = create_request();
        match &mut session.state {
            State::Active { send_method_queue, .. } => {
                send_method_queue.push_back(SendMethod { method: method.to_tokens().unwrap(), channel: tx })
            }
            _ => panic!("session started in the wrong state: {:?}", session.state),
        }

        session.commit_batch(context, CommitBatch);

        let DispatchMessage { address, message, .. } = queue.try_recv().unwrap();
        assert_that!(address, eq(&Address::DeviceSession));
        assert_that!(message, matches_pattern!(Message::SendPacket(_)));
        assert!(queue.is_empty());
        assert_that!(session.state, field!(State::Active.send_method_queue, is_empty()));
        assert_that!(rx.has_message(), eq(false));
    }

    #[tokio::test]
    async fn send_packet_done_success() {
        let session_id = SessionId { hsn: 1, tsn: 2 };
        let mut session = Session::new(session_id, Properties::ASSUMED, Duration::ZERO);
        let (context, queue) = Context::mock();
        let (tx, rx) = oneshot::channel();

        session.send_packet_done(
            context,
            SendPacketDone {
                status: Ok(()),
                methods: vec![WriteQueuedMethod { channel: tx, mgmt_session_meta: None }],
            },
        );

        // Let the timeout task run.
        tokio::task::yield_now().await;

        let DispatchMessage { address, message, .. } = queue.try_recv().unwrap();
        assert_that!(address, eq(&Address::Session(session_id)));
        assert_that!(message, matches_pattern!(Message::Timeout(_)));
        assert!(queue.is_empty());
        assert_that!(session.state, field!(State::Active.channel_queue, len(eq(1))));
        assert_that!(rx.has_message(), eq(false));
    }

    #[tokio::test]
    async fn send_packet_done_failure() {
        let session_id = SessionId { hsn: 1, tsn: 2 };
        let mut session = Session::new(session_id, Properties::ASSUMED, Duration::ZERO);
        let (context, queue) = Context::mock();
        let (tx, rx) = oneshot::channel();

        session.send_packet_done(
            context,
            SendPacketDone {
                status: Err(Error::NotSupported),
                methods: vec![WriteQueuedMethod { channel: tx, mgmt_session_meta: None }],
            },
        );

        // Let the timeout task run (there shouldn't be any timeout task though).
        tokio::task::yield_now().await;

        assert!(queue.is_empty());
        assert_that!(rx.try_recv(), eq(&Ok(Err(Error::NotSupported))));
        assert_that!(session.state, field!(State::Active.channel_queue, is_empty()));
    }

    #[tokio::test]
    async fn packet_received_timeout() {
        let session_id = SessionId { hsn: 1, tsn: 2 };
        let mut session = Session::new(session_id, Properties::ASSUMED, Duration::ZERO);
        let (context, _queue) = Context::mock();
        let (tx, rx) = oneshot::channel();
        let (_cancel_token, cancel_sender) = cancel_channel();

        match &mut session.state {
            State::Active { channel_queue, .. } => channel_queue.push_back(RecvQueuedMethod {
                channel: tx,
                deadline: Instant::now(),
                cancel_sender: Some(cancel_sender),
                mgmt_session_meta: None,
            }),
            _ => panic!("session started in the wrong state: {:?}", session.state),
        }

        session.timeout(context, Instant::now() + Duration::from_secs(1000));

        assert_that!(rx.try_recv(), eq(&Ok(Err(Error::TimedOut))));
        assert_that!(session.state, matches_pattern!(State::Closed));
    }

    #[tokio::test]
    async fn packet_received_receive_regular() {
        let session_id = SessionId { hsn: 1, tsn: 2 };
        let mut session = Session::new(session_id, Properties::ASSUMED, Duration::ZERO);
        let (context, queue) = Context::mock();
        let (tx, rx) = oneshot::channel();
        let (_cancel_token, cancel_sender) = cancel_channel();

        let reply = create_response();
        let packet = session_id.assign(Packet {
            payload: vec![SubPacket {
                kind: SubPacketKind::Data,
                length: PhantomData,
                payload: reply.to_tokens().unwrap(),
            }],
            ..Default::default()
        });

        match &mut session.state {
            State::Active { channel_queue, .. } => channel_queue.push_back(RecvQueuedMethod {
                channel: tx,
                deadline: Instant::now(),
                cancel_sender: Some(cancel_sender),
                mgmt_session_meta: None,
            }),
            _ => panic!("session started in the wrong state: {:?}", session.state),
        }

        session.packet_reveived(context, PacketReceived { packet });

        assert_that!(rx.try_recv(), eq(&Ok(Ok(reply.to_tokens().unwrap()))));
        assert_that!(session.state, field!(State::Active.channel_queue, is_empty()));
        assert!(queue.is_empty());
    }

    #[tokio::test]
    async fn packet_received_receive_eos() {
        let session_id = SessionId { hsn: 1, tsn: 2 };
        let mut session = Session::new(session_id, Properties::ASSUMED, Duration::ZERO);
        let (context, queue) = Context::mock();
        let (tx, rx) = oneshot::channel();
        let (_cancel_token, cancel_sender) = cancel_channel();

        let reply = Command::EndOfSession;
        let packet = session_id.assign(Packet {
            payload: vec![SubPacket {
                kind: SubPacketKind::Data,
                length: PhantomData,
                payload: reply.to_tokens().unwrap(),
            }],
            ..Default::default()
        });

        match &mut session.state {
            State::Active { channel_queue, .. } => channel_queue.push_back(RecvQueuedMethod {
                channel: tx,
                deadline: Instant::now(),
                cancel_sender: Some(cancel_sender),
                mgmt_session_meta: None,
            }),
            _ => panic!("session started in the wrong state: {:?}", session.state),
        }

        session.packet_reveived(context, PacketReceived { packet });

        let DispatchMessage { address, message, .. } = queue.try_recv().unwrap();
        assert_that!(address, eq(&Address::Control));
        assert_that!(message, matches_pattern!(&Message::Delete(_)));
        assert!(queue.is_empty());
        assert_that!(rx.try_recv(), eq(&Ok(Ok(reply.to_tokens().unwrap()))));
        assert_that!(session.state, matches_pattern!(State::Closed));
    }

    #[test]
    fn abort_sends_eos() {
        let session_id = SessionId { hsn: 1, tsn: 2 };
        let mut session = Session::new(session_id, Properties::ASSUMED, TEST_TIMEOUT);
        let (context, queue) = Context::mock();

        session.abort(context);

        // Check if EOS (SendPacket) was sent out.
        let DispatchMessage { address, message, .. } = queue.try_recv().unwrap();
        assert_that!(address, eq(&Address::DeviceSession));
        let Message::SendPacket(SendPacket { packet, .. }) = message else {
            panic!("expected SendPacket, got {:?}", message);
        };
        let payload = packet.payload.into_iter().next().unwrap().payload;
        assert_that!(payload, eq(&Command::EndOfSession.to_tokens().unwrap()));

        // Check if Delete was sent to control.
        let DispatchMessage { address, message, .. } = queue.try_recv().unwrap();
        assert_that!(address, eq(&Address::Control));
        assert_that!(message, matches_pattern!(Message::Delete(_)));

        // Check if no more messages.
        assert!(queue.is_empty());
        assert_that!(session.state, matches_pattern!(State::Closed));
    }

    #[test]
    fn report_aborted_omits_eos() {
        let session_id = SessionId { hsn: 1, tsn: 2 };
        let mut session = Session::new(session_id, Properties::ASSUMED, TEST_TIMEOUT);
        let (context, queue) = Context::mock();

        session.report_aborted(context, ReportAborted);

        // Check if Delete was sent to control.
        let DispatchMessage { address, message, .. } = queue.try_recv().unwrap();
        assert_that!(address, eq(&Address::Control));
        assert_that!(message, matches_pattern!(Message::Delete(_)));

        // Check if no more messages.
        assert!(queue.is_empty());
        assert_that!(session.state, matches_pattern!(State::Closed));
    }
}
