use std::collections::VecDeque;
use std::marker::PhantomData;
use std::time::Instant;

use sed_packet::packet::{PACKET_HEADER_LEN, Packet, SUB_PACKET_HEADER_LEN, SubPacket, SubPacketKind};
use sed_packet::token::{Command, Detokenize, Error as TokenError, SorbitDetokenizer, ToTokens as _};
use sorbit::error::ErrorKind;
use sorbit::io::{FixedMemoryStream, Seek};
use sorbit::stream_ser_de::StreamDeserializer;
use tracing::trace;

use crate::error::Error;
use crate::properties::Properties;
use crate::protocol::message::{CommitBatch, Delete, Message, PacketReceived, SendMethod, SendPacket, SendPacketDone};
use crate::protocol::method::{MethodCallPlaceholder, MethodResultPlaceholder, PendingMethod, retain_alive};
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

    pub fn send_method(&mut self, context: Context, SendMethod { method, channel, span }: SendMethod) {
        let address = self.address();
        self.state = match core::mem::replace(&mut self.state, State::Closed) {
            State::Active { mut send_method_queue, receive_buffer, channel_queue } => {
                trace!(parent: &span, "token to send received");
                let is_end_of_session = Self::is_end_of_session(&method);

                if send_method_queue.is_empty() {
                    context.send(address, Message::CommitBatch(CommitBatch));
                }
                send_method_queue.push_back(SendMethod { method, channel, span });

                if !is_end_of_session {
                    self.commit_batch(context);
                    State::Closing { receive_buffer, channel_queue }
                } else {
                    State::Active { send_method_queue, receive_buffer, channel_queue }
                }
            }
            state @ State::Closing { .. } => {
                let _ = channel.send((Err(Error::Closed), span));
                state
            }
            state @ State::Closed => {
                let _ = channel.send((Err(Error::Closed), span));
                state
            }
        }
    }

    pub fn commit_batch(&mut self, context: Context) {
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
                        let packet = Packet {
                            tper_session_number: self.session_id.tsn,
                            host_session_number: self.session_id.hsn,
                            payload: vec![sub_packet],
                            ..Default::default()
                        };
                        trace!(parent: &span, "wrapped in packet");
                        context.send(
                            topic.clone(),
                            Message::SendPacket(SendPacket {
                                sender: topic.clone(),
                                packet,
                                methods: vec![(channel, span, MethodCallPlaceholder::Session)],
                            }),
                        );
                    } else {
                        let _ = channel.send((Err(Error::MethodTooLarge), span));
                    }
                }
            }
            State::Closing { .. } => (),
            State::Closed => (),
        }
    }

    pub fn send_packet_done(&mut self, context: Context, event: SendPacketDone) {
        let address = self.address();
        match &mut self.state {
            State::Active { channel_queue, .. } | State::Closing { channel_queue, .. } => {
                let deadline = Instant::now() + self.properties.trans_timeout;
                context.send_timeout(address.clone(), deadline);
                for (channel, span, _) in event.methods {
                    channel_queue.push_back(PendingMethod { channel, span, deadline });
                }
            }
            State::Closed => {
                for (channel, span, _) in event.methods {
                    let _ = channel.send((Err(Error::Closed), span));
                }
            }
        }
    }

    pub fn timeout(&mut self, context: Context, time: Instant) {
        match &mut self.state {
            State::Active { channel_queue, .. } | State::Closing { channel_queue, .. } => {
                if retain_alive(time, channel_queue) > 0 {
                    self.abort(context);
                }
            }
            State::Closed => todo!(),
        }
    }

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
                if Self::is_end_of_session(receive_buffer.make_contiguous()) {
                    self.shutdown(context);
                } else {
                    let mut detokenizer = Self::make_detokenizer(receive_buffer);
                    match MethodResultPlaceholder::detokenize(&mut detokenizer) {
                        Ok(_) => {
                            let stream_pos = detokenizer
                                .take()
                                .take()
                                .stream_position()
                                .expect("stream position always succeeds for FixedMemoryStream");
                            let result_tokens: Vec<_> = receive_buffer.drain(..stream_pos as usize).collect();
                            if let Some(PendingMethod { channel, span, .. }) = channel_queue.pop_front() {
                                let _ = channel.send((Ok(result_tokens), span));
                            } else {
                                // Either the device sent too much stuff, or there is a packet distribution bug.
                                self.shutdown(context);
                            }
                        }
                        Err(TokenError::SerializationFailed(err)) if err.kind() == ErrorKind::UnexpectedEof => (),
                        Err(_) => self.shutdown(context),
                    }
                }
            }
            State::Closed => (),
        }
    }

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
                for SendMethod { channel, span, .. } in send_method_queue {
                    let _ = channel.send((Err(Error::Aborted), span));
                }
                for PendingMethod { channel, span, .. } in channel_queue {
                    let _ = channel.send((Err(Error::Aborted), span));
                }
            }
            State::Closing { channel_queue, .. } => {
                for PendingMethod { channel, span, .. } in channel_queue {
                    let _ = channel.send((Err(Error::Aborted), span));
                }
            }
            State::Closed => (),
        };
    }

    fn make_detokenizer(buffer: &mut VecDeque<u8>) -> SorbitDetokenizer<StreamDeserializer<FixedMemoryStream<&[u8]>>> {
        let stream = FixedMemoryStream::new(buffer.make_contiguous() as &[u8]);
        let deserializer = StreamDeserializer::new(stream);
        SorbitDetokenizer::new(deserializer)
    }

    fn is_end_of_session(bytes: &[u8]) -> bool {
        bytes.first() == Command::EndOfSession.to_tokens().expect("tokenization of commands should never fail").first()
    }
}

#[derive(Debug)]
enum State {
    Active {
        send_method_queue: VecDeque<SendMethod>,
        receive_buffer: VecDeque<u8>,
        channel_queue: VecDeque<PendingMethod>,
    },
    Closing {
        receive_buffer: VecDeque<u8>,
        channel_queue: VecDeque<PendingMethod>,
    },
    Closed,
}
