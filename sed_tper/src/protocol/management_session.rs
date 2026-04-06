use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;
use std::time::Instant;

use sed_packet::packet::{PACKET_HEADER_LEN, Packet, SUB_PACKET_HEADER_LEN, SubPacket, SubPacketKind};

use sed_packet::token::{Detokenize as _, Error as TokenError, FromTokens, SorbitDetokenizer};
use sorbit::error::ErrorKind;
use sorbit::io::{FixedMemoryStream, Seek as _};
use sorbit::stream_ser_de::StreamDeserializer;
use tracing::{Span, trace};

use crate::error::Error;
use crate::properties::Properties;
use crate::protocol::messages::{AbortSession, PacketSent, MethodResult, PacketReceived, SendMethod, SendPacket};
use crate::protocol::method_structure::MethodCallPlaceholder;
use crate::protocol::protocol::{Context, Topic};

const SESSION_PROPERTIES: Properties = Properties::ASSUMED;
const MAX_METHOD_SIZE: usize = SESSION_PROPERTIES.max_gross_packet_size - PACKET_HEADER_LEN + SUB_PACKET_HEADER_LEN;

struct ManagementSession {
    send_method_queue: VecDeque<SendMethod>,
    receive_buffer: VecDeque<u8>,
    sync_session_queue: HashMap<u32, VecDeque<(oneshot::Sender<MethodResult>, Span, Instant)>>,
    properties_queue: VecDeque<(oneshot::Sender<MethodResult>, Span, Instant)>,
}

impl ManagementSession {
    fn topic(&self) -> Topic {
        Topic::ManagementLayer
    }

    fn on_send_method(&mut self, context: &mut Context, SendMethod { method, channel, span }: SendMethod) {
        let topic = self.topic();
        trace!(parent: &span, "token to send received");
        let Ok(placeholder) = MethodCallPlaceholder::from_tokens(&method) else {
            let _ = channel.send((Err(Error::MethodCallExpected), span));
            return;
        };
        if method.len() < MAX_METHOD_SIZE {
            let sub_packet = SubPacket { kind: SubPacketKind::Data, length: PhantomData, payload: method };
            let packet = Packet { payload: vec![sub_packet], ..Default::default() };
            trace!(parent: &span, "wrapped in packet");
            context.send(
                topic.clone(),
                SendPacket { source: topic.clone(), packet, methods: vec![(channel, span, placeholder)] },
            );
        } else {
            let _ = channel.send((Err(Error::MethodTooLarge), span));
        }
    }

    fn on_interface_complete(&mut self, _context: &mut Context, event: PacketSent) {
        for (channel, span, placeholder) in event.methods {
            trace!(parent: &span, "sending to interface complete");
            match placeholder {
                MethodCallPlaceholder::StartSession { hsn } => {
                    self.sync_session_queue.entry(hsn).or_default().push_back((channel, span, Instant::now()));
                }
                MethodCallPlaceholder::Properties => {
                    self.properties_queue.push_back((channel, span, Instant::now()));
                }
                _ => (),
            }
        }
    }

    fn on_receive_packet(&mut self, context: &mut Context, PacketReceived { packet }: PacketReceived) {
        assert_eq!(packet.tper_session_number, 0, "the packet was sent to the wrong session");
        assert_eq!(packet.host_session_number, 0, "the packet was sent to the wrong session");

        for SubPacket { kind, payload, .. } in packet.payload {
            if kind == SubPacketKind::Data {
                self.receive_buffer.extend(payload);
            }
        }

        let stream = FixedMemoryStream::new(self.receive_buffer.make_contiguous() as &[u8]);
        let deserializer = StreamDeserializer::new(stream);
        let mut detokenizer = SorbitDetokenizer::new(deserializer);
        match MethodCallPlaceholder::detokenize(&mut detokenizer) {
            Ok(placeholder) => {
                let stream_pos = detokenizer
                    .take()
                    .take()
                    .stream_position()
                    .expect("stream position always succeeds for FixedMemoryStream");
                let method_tokens: Vec<_> = self.receive_buffer.drain(..stream_pos as usize).collect();
                match placeholder {
                    MethodCallPlaceholder::SyncSession { hsn } => {
                        if let Some(pending) = self.sync_session_queue.get_mut(&hsn) {
                            if let Some((channel, span, _)) = pending.pop_front() {
                                trace!(parent: &span, "sent to response channel");
                                let _ = channel.send((Ok(method_tokens), span));
                            }
                            if pending.is_empty() {
                                self.sync_session_queue.remove(&hsn);
                            }
                        }
                    }
                    MethodCallPlaceholder::CloseSession { hsn, tsn } => {
                        context.send(Topic::SessionLayer { tsn, hsn }, AbortSession);
                    }
                    MethodCallPlaceholder::Properties => {
                        if let Some((channel, span, _)) = self.properties_queue.pop_front() {
                            trace!(parent: &span, "sent to response channel");
                            let _ = channel.send((Ok(method_tokens), span));
                        }
                    }
                    _ => (),
                }
            }
            Err(TokenError::SerializationFailed(err)) if err.kind() == ErrorKind::UnexpectedEof => (),
            Err(_) => self.reset(),
        }
    }

    fn reset(&mut self) {
        self.receive_buffer.clear();
        for pending in core::mem::replace(&mut self.sync_session_queue, HashMap::new()).into_values() {
            for (channel, span, _) in pending {
                let _ = channel.send((Err(Error::Aborted), span));
            }
        }
    }
}
