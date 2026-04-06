use std::collections::VecDeque;
use std::marker::PhantomData;

use sed_packet::packet::{PACKET_HEADER_LEN, Packet, SUB_PACKET_HEADER_LEN, SubPacket, SubPacketKind};
use sed_packet::token::{Command, Detokenize, Detokenizer, Error as TokenError, SorbitDetokenizer, ToTokens as _};
use sorbit::error::ErrorKind;
use sorbit::io::{FixedMemoryStream, Seek};
use sorbit::stream_ser_de::StreamDeserializer;
use tracing::{Span, trace};

use crate::error::Error;
use crate::properties::Properties;
use crate::protocol::messages::{
    AbortSession, AssemblePacket, MethodResult, PacketSent, PacketReceived, RemoveSession, SendMethod, SendPacket,
};
use crate::protocol::method_structure::{MethodCallPlaceholder, MethodResultPlaceholder};
use crate::protocol::protocol::{Context, Topic};

pub struct Session {
    tsn: u32,
    hsn: u32,
    properties: Properties,
    state: State,
}

impl Session {
    fn topic(&self) -> Topic {
        Topic::SessionLayer { tsn: self.tsn, hsn: self.hsn }
    }

    fn on_send_method(&mut self, context: &mut Context, SendMethod { method, channel, span }: SendMethod) {
        let topic = self.topic();
        self.state = match core::mem::replace(&mut self.state, State::Closed) {
            State::Active { mut send_method_queue, receive_buffer, channel_queue } => {
                trace!(parent: &span, "token to send received");
                let is_end_of_session = Self::is_end_of_session(&method);

                if send_method_queue.is_empty() {
                    context.send(topic, AssemblePacket);
                }
                send_method_queue.push_back(SendMethod { method, channel, span });

                if !is_end_of_session {
                    self.on_assemble_packet(context, AssemblePacket);
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

    fn on_assemble_packet(&mut self, context: &mut Context, _event: AssemblePacket) {
        let topic = self.topic();
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
                            tper_session_number: self.tsn,
                            host_session_number: self.hsn,
                            payload: vec![sub_packet],
                            ..Default::default()
                        };
                        trace!(parent: &span, "wrapped in packet");
                        context.send(
                            topic.clone(),
                            SendPacket {
                                source: topic.clone(),
                                packet,
                                methods: vec![(channel, span, MethodCallPlaceholder::Session)],
                            },
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

    fn on_interface_complete(&mut self, _context: &mut Context, event: PacketSent) {
        match &mut self.state {
            State::Active { channel_queue, .. } | State::Closing { channel_queue, .. } => {
                for (channel, span, _) in event.methods {
                    channel_queue.push_back((channel, span));
                }
            }
            State::Closed => {
                for (channel, span, _) in event.methods {
                    let _ = channel.send((Err(Error::Closed), span));
                }
            }
        }
    }

    fn on_receive_packet(&mut self, context: &mut Context, PacketReceived { packet }: PacketReceived) {
        match &mut self.state {
            State::Active { receive_buffer, channel_queue, .. }
            | State::Closing { receive_buffer, channel_queue, .. } => {
                assert_eq!(packet.tper_session_number, self.tsn, "the packet was sent to the wrong session");
                assert_eq!(packet.host_session_number, self.hsn, "the packet was sent to the wrong session");

                for SubPacket { kind, payload, .. } in packet.payload {
                    if kind == SubPacketKind::Data {
                        receive_buffer.extend(payload);
                    }
                }
                if Self::is_end_of_session(receive_buffer.make_contiguous()) {
                    self.finalize(context);
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
                            if let Some((channel, span)) = channel_queue.pop_front() {
                                let _ = channel.send((Ok(result_tokens), span));
                            } else {
                                // Either the device sent too much stuff, or there is a packet distribution bug.
                                self.finalize(context);
                            }
                        }
                        Err(TokenError::SerializationFailed(err)) if err.kind() == ErrorKind::UnexpectedEof => (),
                        Err(_) => self.finalize(context),
                    }
                }
            }
            State::Closed => (),
        }
    }

    pub fn on_abort(&mut self, context: &mut Context, _message: AbortSession) {
        self.finalize(context);
    }

    fn finalize(&mut self, context: &mut Context) {
        match core::mem::replace(&mut self.state, State::Closed) {
            State::Active { send_method_queue, channel_queue, .. } => {
                // Send an END_OF_SESSION to the TPer.
                let packet = Packet {
                    tper_session_number: self.tsn,
                    host_session_number: self.hsn,
                    payload: vec![SubPacket {
                        kind: SubPacketKind::Data,
                        length: PhantomData,
                        payload: Command::EndOfSession.to_tokens().expect("serializing a command should never fail"),
                    }],
                    ..Default::default()
                };
                context.send(self.topic(), SendPacket { source: self.topic(), packet, methods: vec![] });

                for SendMethod { channel, span, .. } in send_method_queue {
                    let _ = channel.send((Err(Error::Aborted), span));
                }
                for (channel, span) in channel_queue {
                    let _ = channel.send((Err(Error::Aborted), span));
                }
            }
            State::Closing { channel_queue, .. } => {
                for (channel, span) in channel_queue {
                    let _ = channel.send((Err(Error::Aborted), span));
                }
            }
            State::Closed => (),
        };
        context.send(Topic::Stack, RemoveSession { tsn: self.tsn, hsn: self.hsn });
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

enum State {
    Active {
        send_method_queue: VecDeque<SendMethod>,
        receive_buffer: VecDeque<u8>,
        channel_queue: VecDeque<(oneshot::Sender<MethodResult>, Span)>,
    },
    Closing {
        receive_buffer: VecDeque<u8>,
        channel_queue: VecDeque<(oneshot::Sender<MethodResult>, Span)>,
    },
    Closed,
}
