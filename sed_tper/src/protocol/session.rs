use std::collections::VecDeque;
use std::marker::PhantomData;
use std::time::Instant;

use sed_async_runtime::cancel_channel;
use sed_packet::packet::{PACKET_HEADER_LEN, Packet, SUB_PACKET_HEADER_LEN, SubPacket, SubPacketKind};
use sed_packet::token::{Command, Detokenize, Error as TokenError, SorbitDetokenizer, ToTokens as _};
use sed_spec::methods::Properties;
use sorbit::error::ErrorKind;
use sorbit::io::{FixedMemoryStream, Seek};
use sorbit::stream_ser_de::StreamDeserializer;
use tracing::{Span, instrument, trace};

use crate::error::Error;
use crate::protocol::message::{CommitBatch, Delete, Message, PacketReceived, SendMethod, SendPacket, SendPacketDone};
use crate::protocol::method::{AnyMethodResult, RecvQueuedMethod, WriteQueuedMethod, retain_alive};
use crate::protocol::protocol::{Address, Context};
use crate::protocol::session_id::SessionId;

#[derive(Debug)]
pub struct Session {
    session_id: SessionId,
    properties: Properties,
    state: State,
}

impl Session {
    pub fn new(session_id: SessionId, properties: Properties) -> Self {
        Self {
            session_id,
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

    #[instrument(level = "debug")]
    pub fn send_method(&mut self, context: Context, SendMethod { method, channel, span }: SendMethod) {
        let address = self.address();
        self.state = match core::mem::replace(&mut self.state, State::Closed) {
            State::Active { mut send_method_queue, receive_buffer, channel_queue } => {
                trace!(parent: &span, "token to send received");
                let is_end_of_session = is_end_of_session(&method);

                if send_method_queue.is_empty() {
                    context.send(address, Message::CommitBatch(CommitBatch(Span::current())));
                }
                send_method_queue.push_back(SendMethod { method, channel, span });

                if !is_end_of_session {
                    self.commit_batch(context, CommitBatch(Span::current()));
                    State::Active { send_method_queue, receive_buffer, channel_queue }
                } else {
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

    #[instrument(level = "debug")]
    pub fn commit_batch(&mut self, context: Context, message: CommitBatch) {
        Span::current().follows_from(message.0);
        let topic = self.address();
        match &mut self.state {
            State::Active { send_method_queue, .. } => {
                let max_method_size =
                    std::cmp::max(PACKET_HEADER_LEN + SUB_PACKET_HEADER_LEN, self.properties.max_gross_packet_size)
                        - PACKET_HEADER_LEN
                        + SUB_PACKET_HEADER_LEN;
                while let Some(SendMethod { method, channel, span }) = send_method_queue.pop_front() {
                    if method.len() <= max_method_size {
                        let sub_packet = SubPacket { kind: SubPacketKind::Data, length: PhantomData, payload: method };
                        let packet = self.session_id.assign(Packet { payload: vec![sub_packet], ..Default::default() });
                        trace!(parent: &span, "wrapped in packet");
                        context.send(
                            Address::DeviceSession,
                            Message::SendPacket(SendPacket {
                                sender: topic.clone(),
                                packet,
                                methods: vec![WriteQueuedMethod { channel, span, mgmt_session_meta: None }],
                            }),
                        );
                    } else {
                        let _ = channel.send(Err(Error::MethodTooLarge));
                    }
                }
            }
            State::Closing { .. } => (),
            State::Closed => (),
        }
    }

    #[instrument(level = "debug")]
    pub fn send_packet_done(&mut self, context: Context, SendPacketDone { status, methods }: SendPacketDone) {
        let address = self.address();
        match &mut self.state {
            State::Active { channel_queue, .. } | State::Closing { channel_queue, .. } => match &status {
                Ok(_) => {
                    let deadline = Instant::now() + self.properties.trans_timeout;
                    let (cancel_token, cancel_sender) = cancel_channel();
                    context.send_timeout(address.clone(), deadline, Some(cancel_token));
                    for WriteQueuedMethod { channel, span, .. } in methods {
                        channel_queue.push_back(RecvQueuedMethod {
                            channel,
                            span,
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

    #[instrument(level = "debug")]
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

    #[instrument(level = "debug")]
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
                if is_end_of_session(receive_buffer.make_contiguous()) {
                    self.shutdown(context);
                } else {
                    let mut detokenizer = make_detokenizer(receive_buffer);
                    match AnyMethodResult::detokenize(&mut detokenizer) {
                        Ok(_) => {
                            let stream_pos = detokenizer
                                .take()
                                .take()
                                .stream_position()
                                .expect("stream position always succeeds for FixedMemoryStream");
                            let result_tokens: Vec<_> = receive_buffer.drain(..stream_pos as usize).collect();
                            if let Some(RecvQueuedMethod { channel, .. }) = channel_queue.pop_front() {
                                let _ = channel.send(Ok(result_tokens));
                            } else {
                                // Either the device sent too much stuff, or there is a packet distribution bug.
                                self.abort(context);
                            }
                        }
                        Err(TokenError::SerializationFailed(err)) if err.kind() == ErrorKind::UnexpectedEof => (),
                        Err(_) => self.abort(context),
                    }
                }
            }
            State::Closed => (),
        }
    }

    #[instrument(level = "debug")]
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
                    self.address(),
                    Message::SendPacket(SendPacket { sender: self.address(), packet, methods: vec![] }),
                );
            }
            _ => (),
        };
        self.shutdown(context);
    }

    fn shutdown(&mut self, context: Context) {
        self.abort_pending_methods();
        context.send(Address::Control, Message::Delete(Delete(self.session_id)));
    }

    fn abort_pending_methods(&mut self) {
        match core::mem::replace(&mut self.state, State::Closed) {
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

fn make_detokenizer(buffer: &mut VecDeque<u8>) -> SorbitDetokenizer<StreamDeserializer<FixedMemoryStream<&[u8]>>> {
    let stream = FixedMemoryStream::new(buffer.make_contiguous() as &[u8]);
    let deserializer = StreamDeserializer::new(stream);
    SorbitDetokenizer::new(deserializer)
}

fn is_end_of_session(bytes: &[u8]) -> bool {
    bytes.first() == Command::EndOfSession.to_tokens().expect("tokenization of commands should never fail").first()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    use googletest::{assert_that, prelude::*};
    use sed_packet::token::ToTokens;
    use sed_spec::methods::{Activate, ActivateResult, MethodCall, MethodResult, MethodStatus};
    use sed_spec::preconfig::core::shared::method_id::ACTIVATE;
    use sed_spec::preconfig::opal_2::admin::sp;
    use tracing::Span;

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
    fn send_method() {
        let session_id = SessionId { hsn: 1, tsn: 2 };
        let mut session = Session::new(session_id, Properties::ASSUMED);
        let (context, queue) = Context::mock();
        let (tx, rx) = oneshot::channel();
        let method = create_request();
        session.send_method(
            context,
            SendMethod { method: method.to_tokens().unwrap(), channel: tx, span: Span::current() },
        );

        let (_, address, content) = queue.try_recv().unwrap();
        assert_that!(address, eq(&Address::Session(session_id)));
        assert_that!(content, matches_pattern!(Message::CommitBatch(_)));
        assert!(queue.is_empty());
        assert_that!(session.state, field!(State::Active.send_method_queue, len(eq(1))));
        assert_that!(rx.has_message(), eq(false));
    }

    #[test]
    fn commit_batch() {
        let session_id = SessionId { hsn: 1, tsn: 2 };
        let mut session = Session::new(session_id, Properties::ASSUMED);
        let (context, queue) = Context::mock();
        let (tx, rx) = oneshot::channel();
        let method = create_request();
        match &mut session.state {
            State::Active { send_method_queue, .. } => send_method_queue.push_back(SendMethod {
                method: method.to_tokens().unwrap(),
                channel: tx,
                span: Span::current(),
            }),
            _ => panic!("session started in the wrong state: {:?}", session.state),
        }

        session.commit_batch(context, CommitBatch(Span::current()));

        let (_, address, content) = queue.try_recv().unwrap();
        assert_that!(address, eq(&Address::DeviceSession));
        assert_that!(content, matches_pattern!(Message::SendPacket(_)));
        assert!(queue.is_empty());
        assert_that!(session.state, field!(State::Active.send_method_queue, is_empty()));
        assert_that!(rx.has_message(), eq(false));
    }

    #[tokio::test]
    async fn send_packet_done_success() {
        let session_id = SessionId { hsn: 1, tsn: 2 };
        let mut session = Session::new(
            session_id,
            Properties { trans_timeout: Duration::ZERO, def_trans_timeout: Duration::ZERO, ..Properties::ASSUMED },
        );
        let (context, queue) = Context::mock();
        let (tx, rx) = oneshot::channel();

        session.send_packet_done(
            context,
            SendPacketDone {
                status: Ok(()),
                methods: vec![WriteQueuedMethod { channel: tx, span: Span::current(), mgmt_session_meta: None }],
            },
        );

        // Let the timeout task run.
        tokio::task::yield_now().await;

        let (_, address, content) = queue.try_recv().unwrap();
        assert_that!(address, eq(&Address::Session(session_id)));
        assert_that!(content, matches_pattern!(Message::Timeout(_)));
        assert!(queue.is_empty());
        assert_that!(session.state, field!(State::Active.channel_queue, len(eq(1))));
        assert_that!(rx.has_message(), eq(false));
    }

    #[tokio::test]
    async fn send_packet_done_failure() {
        let session_id = SessionId { hsn: 1, tsn: 2 };
        let mut session = Session::new(
            session_id,
            Properties { trans_timeout: Duration::ZERO, def_trans_timeout: Duration::ZERO, ..Properties::ASSUMED },
        );
        let (context, queue) = Context::mock();
        let (tx, rx) = oneshot::channel();

        session.send_packet_done(
            context,
            SendPacketDone {
                status: Err(Error::NotSupported),
                methods: vec![WriteQueuedMethod { channel: tx, span: Span::current(), mgmt_session_meta: None }],
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
        let mut session = Session::new(
            session_id,
            Properties { trans_timeout: Duration::ZERO, def_trans_timeout: Duration::ZERO, ..Properties::ASSUMED },
        );
        let (context, _queue) = Context::mock();
        let (tx, rx) = oneshot::channel();
        let (_cancel_token, cancel_sender) = cancel_channel();

        match &mut session.state {
            State::Active { channel_queue, .. } => channel_queue.push_back(RecvQueuedMethod {
                channel: tx,
                span: Span::current(),
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
    async fn packet_received_receive() {
        let session_id = SessionId { hsn: 1, tsn: 2 };
        let mut session = Session::new(
            session_id,
            Properties { trans_timeout: Duration::ZERO, def_trans_timeout: Duration::ZERO, ..Properties::ASSUMED },
        );
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
                span: Span::current(),
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
}
