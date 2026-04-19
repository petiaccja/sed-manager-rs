use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;
use std::time::Instant;

use sed_packet::packet::{PACKET_HEADER_LEN, Packet, SUB_PACKET_HEADER_LEN, SubPacket, SubPacketKind};

use sed_packet::token::{Detokenize as _, Error as TokenError, FromTokens as _, SorbitDetokenizer};
use sed_spec::methods::{
    CloseSession, MethodStatus, MgmtMethodCall, MgmtMethodCallParams, Properties, PropertiesMethod, SyncSession,
};
use sorbit::error::ErrorKind;
use sorbit::io::{FixedMemoryStream, Seek as _};
use sorbit::stream_ser_de::StreamDeserializer;
use tracing::trace;

use crate::error::Error;
use crate::protocol::message::{Abort, Message, PacketReceived, SendMethod, SendPacket, SendPacketDone, Spawn};
use crate::protocol::method::{MgmtMethodCallSend, RecvQueuedMethod, WriteQueuedMethod, retain_alive};
use crate::protocol::protocol::{Address, Context};
use crate::protocol::session_id::SessionId;

const MAX_METHOD_SIZE: usize = Properties::ASSUMED.max_gross_packet_size - PACKET_HEADER_LEN + SUB_PACKET_HEADER_LEN;

#[derive(Debug)]
pub struct ManagementSession {
    capabilities: Properties,
    properties: Properties,
    receive_buffer: VecDeque<u8>,
    sync_session_queue: HashMap<u32, VecDeque<RecvQueuedMethod>>,
}

impl ManagementSession {
    const ADDRESS: Address = Address::ManagementSession;

    pub fn new(capabilities: Properties) -> Self {
        Self {
            capabilities,
            properties: Properties::ASSUMED,
            receive_buffer: VecDeque::new(),
            sync_session_queue: HashMap::new(),
        }
    }

    pub fn send_method(&mut self, context: Context, SendMethod { method, channel, span }: SendMethod) {
        trace!(parent: &span, "token to send received");
        let Ok(call) = MgmtMethodCallSend::from_tokens(&method) else {
            let _ = channel.send(Err(Error::MethodCallExpected));
            return;
        };
        if method.len() < MAX_METHOD_SIZE {
            let sub_packet = SubPacket { kind: SubPacketKind::Data, length: PhantomData, payload: method };
            let packet = Packet { payload: vec![sub_packet], ..Default::default() };
            trace!(parent: &span, "wrapped in packet");
            context.send(
                Address::DeviceSession,
                Message::SendPacket(SendPacket {
                    sender: Self::ADDRESS,
                    packet,
                    methods: vec![WriteQueuedMethod {
                        channel,
                        span,
                        mgmt_session_meta: Some((call, self.properties.clone()).into()),
                    }],
                }),
            );
        } else {
            let _ = channel.send(Err(Error::MethodTooLarge));
        }
    }

    pub fn send_packet_done(&mut self, context: Context, SendPacketDone { status, methods }: SendPacketDone) {
        for WriteQueuedMethod { channel, span, mgmt_session_meta } in methods {
            match &status {
                Ok(_) => {
                    trace!(parent: &span, "containing packet sent to the device succesfully");
                    let mgmt_session_meta =
                        mgmt_session_meta.expect("SM methods must always have meta data, wrong address?");
                    match mgmt_session_meta.0 {
                        MgmtMethodCallSend::StartSession { hsn } => {
                            let deadline = Instant::now() + self.capabilities.def_trans_timeout;
                            context.send_timeout(Self::ADDRESS, deadline);
                            self.sync_session_queue.entry(hsn).or_default().push_back(RecvQueuedMethod {
                                channel,
                                span,
                                deadline,
                                mgmt_session_meta: Some(mgmt_session_meta.1.into()),
                            });
                        }
                        _ => (),
                    }
                }
                Err(err) => {
                    let _ = channel.send(Err(err.clone()));
                }
            }
        }
    }

    pub fn timeout(&mut self, time: Instant) {
        for (_, queue) in &mut self.sync_session_queue {
            retain_alive(time, queue);
        }
        self.sync_session_queue.retain(|_, queue| !queue.is_empty());
    }

    pub fn packet_received(&mut self, context: Context, PacketReceived { packet }: PacketReceived) {
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
        match MgmtMethodCall::detokenize(&mut detokenizer) {
            Ok(response) => {
                let stream_pos = detokenizer
                    .take()
                    .take()
                    .stream_position()
                    .expect("stream position always succeeds for FixedMemoryStream");
                let method_tokens: Vec<_> = self.receive_buffer.drain(..stream_pos as usize).collect();
                match &response.params {
                    MgmtMethodCallParams::SyncSession(SyncSession { host_session_id, sp_session_id, .. }) => {
                        if let Some(pending) = self.sync_session_queue.get_mut(&host_session_id) {
                            if let Some(RecvQueuedMethod { channel, span, mgmt_session_meta, .. }) = pending.pop_front()
                            {
                                let properties =
                                    *mgmt_session_meta.expect("SM methods must always have meta data, wrong address?");
                                trace!(parent: &span, "sent to response channel");
                                let _ = channel.send(Ok(method_tokens));
                                if response.status == MethodStatus::Success {
                                    context.send(
                                        Address::Control,
                                        Message::Spawn(Spawn {
                                            id: SessionId { hsn: *host_session_id, tsn: *sp_session_id },
                                            properties,
                                        }),
                                    );
                                }
                            }
                            if pending.is_empty() {
                                self.sync_session_queue.remove(&host_session_id);
                            }
                        }
                    }
                    MgmtMethodCallParams::CloseSession(CloseSession {
                        remote_session_number,
                        local_session_number,
                    }) => {
                        context.send(
                            Address::Session(SessionId { hsn: *remote_session_number, tsn: *local_session_number }),
                            Message::Abort(Abort),
                        );
                    }
                    MgmtMethodCallParams::Properties(properties) => match properties {
                        PropertiesMethod::Host { .. } => (),
                        PropertiesMethod::TPer { properties, .. } => {
                            self.properties = Properties::common(&self.capabilities, properties)
                        }
                    },
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
            for RecvQueuedMethod { channel, .. } in pending {
                let _ = channel.send(Err(Error::Aborted));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::Duration;

    use googletest::{assert_that, prelude::*};
    use sed_packet::token::ToTokens;
    use sed_spec::methods::{MethodCall, StartSession};
    use sed_spec::preconfig::core::shared::invoking_id::SESSION_MANAGER;
    use sed_spec::preconfig::core::shared::sm_method_id::{START_SESSION, SYNC_SESSION};
    use sed_spec::preconfig::opal_2::admin::sp;
    use tracing::Span;

    fn create_request() -> MethodCall<StartSession> {
        MethodCall {
            invoking_id: SESSION_MANAGER,
            method_id: START_SESSION,
            parameters: StartSession {
                host_session_id: 1,
                spid: sp::ADMIN,
                write: true,
                host_challenge: None,
                host_exchange_authority: None,
                host_exchange_cert: None,
                host_signing_authority: None,
                host_signing_cert: None,
                session_timeout: None,
                trans_timeout: None,
                initial_credit: None,
                signed_hash: None,
            },
            status: MethodStatus::Success,
        }
    }

    fn create_response() -> MethodCall<SyncSession> {
        MethodCall {
            invoking_id: SESSION_MANAGER,
            method_id: SYNC_SESSION,
            parameters: SyncSession {
                host_session_id: 1,
                sp_session_id: 2,
                sp_challenge: None,
                sp_exchange_cert: None,
                sp_signing_cert: None,
                trans_timeout: None,
                initial_credit: None,
                signed_hash: None,
            },
            status: MethodStatus::Success,
        }
    }

    #[test]
    fn send_method() {
        let mut mgmt_session = ManagementSession::new(Properties::ASSUMED);
        let (context, queue) = Context::mock();
        let (tx, rx) = oneshot::channel();
        let method = create_request();
        mgmt_session.send_method(
            context,
            SendMethod { method: method.to_tokens().unwrap(), channel: tx, span: Span::current() },
        );

        let (address, content) = queue.try_recv().unwrap();
        assert_that!(address, eq(&Address::DeviceSession));
        assert_that!(content, matches_pattern!(Message::SendPacket(_)));
        assert!(queue.is_empty());
        assert_that!(rx.has_message(), eq(false));
    }

    #[tokio::test]
    async fn send_packet_done_success() {
        let mut mgmt_session = ManagementSession::new(Properties {
            trans_timeout: Duration::ZERO,
            def_trans_timeout: Duration::ZERO,
            ..Properties::ASSUMED
        });
        let (context, queue) = Context::mock();
        let (tx, rx) = oneshot::channel();

        mgmt_session.send_packet_done(
            context,
            SendPacketDone {
                status: Ok(()),
                methods: vec![WriteQueuedMethod {
                    channel: tx,
                    span: Span::current(),
                    mgmt_session_meta: Some((MgmtMethodCallSend::StartSession { hsn: 1 }, Properties::ASSUMED).into()),
                }],
            },
        );

        // Let the timeout task run.
        tokio::task::yield_now().await;

        let (address, content) = queue.try_recv().unwrap();
        assert_that!(address, eq(&Address::ManagementSession));
        assert_that!(content, matches_pattern!(Message::Timeout(_)));
        assert!(queue.is_empty());
        assert_that!(rx.has_message(), eq(false));
        assert_that!(&mgmt_session.sync_session_queue, has_entry(1, anything()));
    }

    #[tokio::test]
    async fn send_packet_done_failure() {
        let mut mgmt_session = ManagementSession::new(Properties {
            trans_timeout: Duration::ZERO,
            def_trans_timeout: Duration::ZERO,
            ..Properties::ASSUMED
        });
        let (context, queue) = Context::mock();
        let (tx, rx) = oneshot::channel();

        mgmt_session.send_packet_done(
            context,
            SendPacketDone {
                status: Err(Error::NotSupported),
                methods: vec![WriteQueuedMethod {
                    channel: tx,
                    span: Span::current(),
                    mgmt_session_meta: Some((MgmtMethodCallSend::StartSession { hsn: 1 }, Properties::ASSUMED).into()),
                }],
            },
        );

        // Let the timeout task run (there shouldn't be any timeout task though).
        tokio::task::yield_now().await;

        assert!(queue.is_empty());
        assert_that!(rx.try_recv(), eq(&Ok(Err(Error::NotSupported))));
        assert_that!(&mgmt_session.sync_session_queue, is_empty());
    }

    #[tokio::test]
    async fn packet_received_timeout() {
        let mut mgmt_session = ManagementSession::new(Properties {
            trans_timeout: Duration::ZERO,
            def_trans_timeout: Duration::ZERO,
            ..Properties::ASSUMED
        });
        let (tx, rx) = oneshot::channel();

        mgmt_session.sync_session_queue.entry(1).or_default().push_back(RecvQueuedMethod {
            channel: tx,
            span: Span::current(),
            deadline: Instant::now(),
            mgmt_session_meta: Some(Properties::ASSUMED.into()),
        });

        mgmt_session.timeout(Instant::now() + Duration::from_secs(1000));

        assert_that!(rx.try_recv(), eq(&Ok(Err(Error::TimedOut))));
        assert_that!(&mgmt_session.sync_session_queue, is_empty());
    }

    #[tokio::test]
    async fn packet_received_receive() {
        let mut mgmt_session = ManagementSession::new(Properties {
            trans_timeout: Duration::ZERO,
            def_trans_timeout: Duration::ZERO,
            ..Properties::ASSUMED
        });
        let (context, queue) = Context::mock();
        let (tx, rx) = oneshot::channel();

        let reply = create_response();
        let packet = SessionId { hsn: 0, tsn: 0 }.assign(Packet {
            payload: vec![SubPacket {
                kind: SubPacketKind::Data,
                length: PhantomData,
                payload: reply.to_tokens().unwrap(),
            }],
            ..Default::default()
        });

        mgmt_session.sync_session_queue.entry(1).or_default().push_back(RecvQueuedMethod {
            channel: tx,
            span: Span::current(),
            deadline: Instant::now(),
            mgmt_session_meta: Some(Properties::ASSUMED.into()),
        });

        mgmt_session.packet_received(context, PacketReceived { packet });

        assert_that!(rx.try_recv(), eq(&Ok(Ok(reply.to_tokens().unwrap()))));
        assert_that!(&mgmt_session.sync_session_queue, is_empty());

        let (address, content) = queue.try_recv().unwrap();
        assert_that!(address, eq(&Address::Control));
        assert_that!(content, matches_pattern!(Message::Spawn(_)));
        assert!(queue.is_empty());
    }
}
