//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::{
    collections::{HashMap, VecDeque},
    time::{Duration, Instant},
};

use oneshot::Sender;
use sed_packet::{
    packet::{PACKET_HEADER_LEN, Packet, SUB_PACKET_HEADER_LEN},
    session_id::SessionId,
    token_stream::ToTokens,
};
use sed_spec::{
    methods::{
        CloseSession, ExtractResult, MethodCall, MethodParam, MethodStatus, MgmtMethodCall, MgmtMethodCallParams,
        Properties, PropertiesMethod, StartSession, SyncSession, extract_method,
    },
    preconfig::core::shared::{invoking_id::SESSION_MANAGER, sm_method_id::START_SESSION},
};
use tracing::Span;

use crate::{
    Error,
    protocol::{
        sequence_number::SequenceNumber,
        shared::{PacketBatch, PropertiesChanged, link_both_ways, min_deadline, packetize_one},
    },
};

#[derive(Debug)]
pub struct Management {
    sequence_number: SequenceNumber,
    timeout: Duration,
    capabilities: Properties,
    properties: Properties,
    start_session_calls_enqueued: VecDeque<StartSessionCallRecord>,
    /// `StartSession` methods that have been submitted for IF-SEND, keyed by HSN.
    start_session_calls_sending: HashMap<u32, VecDeque<StartSessionSendingRecord>>,
    start_session_calls_receiving: HashMap<u32, VecDeque<StartSessionReceivingRecord>>,
    received_tokens: VecDeque<u8>,
    properties_call: PropertiesCall,
    properties_changed_tx: async_broadcast::Sender<PropertiesChanged>,
    properties_changed_rx: async_broadcast::InactiveReceiver<PropertiesChanged>,
}

impl Management {
    pub fn new(timeout: Duration, capabilities: Properties) -> Self {
        let (properties_changed_tx, properties_changed_rx) = async_broadcast::broadcast(1);
        Self {
            sequence_number: SequenceNumber::initial(),
            timeout,
            capabilities,
            properties: Properties::INITIAL,
            start_session_calls_enqueued: VecDeque::new(),
            start_session_calls_sending: HashMap::new(),
            start_session_calls_receiving: HashMap::new(),
            received_tokens: VecDeque::new(),
            properties_call: PropertiesCall::Idle,
            properties_changed_tx,
            properties_changed_rx: properties_changed_rx.deactivate(),
        }
    }

    pub fn handle_call_start_session(
        &mut self,
        call: StartSession,
        sender: Sender<Result<(SyncSession, Properties), Error>>,
        span: Span,
    ) {
        self.start_session_calls_enqueued.push_back(StartSessionCallRecord { call, sender, span });
    }

    /// Request synchronizing the connection properties with the TPer. The
    /// request is ignored while another one is already outstanding.
    pub fn handle_call_properties(&mut self, span: Span) {
        if let PropertiesCall::Idle = self.properties_call {
            self.properties_call = PropertiesCall::Requested { span };
        } else {
            tracing::debug!(parent: &span, "ignored: another properties sync is outstanding");
        }
    }

    pub fn handle_iface_send_done(&mut self, time: Instant, sn: SequenceNumber, result: Result<(), Error>) {
        for (hsn, queue) in &mut self.start_session_calls_sending {
            while let Some(record) = queue.pop_front_if(|record| record.sequence_number <= sn) {
                let deadline = time + self.timeout;
                match &result {
                    Ok(_) => {
                        let record = StartSessionReceivingRecord {
                            sent_at: time,
                            deadline,
                            properties: record.properties,
                            sender: record.sender,
                            span: record.span,
                        };
                        self.start_session_calls_receiving.entry(*hsn).or_default().push_back(record);
                    }
                    Err(err) => {
                        let _ = record.sender.send(Err(err.clone()));
                    }
                };
            }
        }
        self.start_session_calls_sending.retain(|_, queue| !queue.is_empty());

        self.properties_call = match core::mem::replace(&mut self.properties_call, PropertiesCall::Idle) {
            PropertiesCall::Sending { sequence_number, span } if sequence_number <= sn => match &result {
                Ok(_) => PropertiesCall::Receiving { sent_at: time, deadline: time + self.timeout, span },
                Err(_) => PropertiesCall::Idle,
            },
            state => state,
        };
    }

    pub fn handle_reset(&mut self) {
        // Create a new instance from scratch.
        let timeout = self.timeout;
        let capabilities = self.capabilities.clone();
        let current = core::mem::replace(self, Self::new(timeout, capabilities));

        // Drop and finalize current resources.
        let (properties_changed_tx, properties_changed_rx) = {
            let Self {
                start_session_calls_enqueued: method_calls,
                start_session_calls_sending,
                start_session_calls_receiving,
                properties_changed_tx,
                properties_changed_rx,
                ..
            } = current;
            for StartSessionCallRecord { sender, .. } in method_calls {
                let _ = sender.send(Err(Error::Aborted));
            }
            for (_, queue) in start_session_calls_sending {
                for StartSessionSendingRecord { sender, .. } in queue {
                    let _ = sender.send(Err(Error::Aborted));
                }
            }
            for (_, queue) in start_session_calls_receiving {
                for StartSessionReceivingRecord { sender, .. } in queue {
                    let _ = sender.send(Err(Error::Aborted));
                }
            }
            (properties_changed_tx, properties_changed_rx)
        };

        // Save transferable resources.
        self.properties_changed_tx = properties_changed_tx;
        self.properties_changed_rx = properties_changed_rx;

        // Inform that properties have been reset.
        let _ = self.properties_changed_tx.try_broadcast(PropertiesChanged {
            remote_properties: Properties::INITIAL,
            connection_properties: Properties::INITIAL,
        });
    }

    /// Process the tokens returned by the device.
    ///
    /// # Parameters
    ///
    /// - `tokens`: the tokens returned by the IF-RECV command.
    /// - `source`: the span of the IF-RECV command that returned the tokens.
    #[must_use]
    pub fn handle_tokens(&mut self, tokens: Vec<u8>, source: &Span) -> Vec<StackAction> {
        let mut actions = Vec::new();
        self.received_tokens.extend(tokens);
        loop {
            match extract_method::<MgmtMethodCall>(&mut self.received_tokens) {
                ExtractResult::Ok { value, .. } => match value.params {
                    // The host should never receive a `StartSession`.
                    MgmtMethodCallParams::StartSession(_) => (),
                    MgmtMethodCallParams::SyncSession(sync_session) => {
                        self.handle_sync_session(&mut actions, sync_session, value.status, source)
                    }
                    MgmtMethodCallParams::CloseSession(close_session) => {
                        Self::handle_close_session(&mut actions, close_session, value.status);
                    }
                    MgmtMethodCallParams::Properties(properties_method) => {
                        if let PropertiesCall::Receiving { span, .. } = &self.properties_call {
                            link_both_ways(source, span);
                            self.properties_call = PropertiesCall::Idle;
                        }
                        self.handle_properties(properties_method, value.status)
                    }
                },
                ExtractResult::EndOfSession => (),
                ExtractResult::NeedMoreTokens => break,
                ExtractResult::InvalidTokens(error) => {
                    self.flush(error.into());
                    break;
                }
            };
        }
        actions
    }

    pub fn poll_action(&mut self, time: Instant) -> Action {
        // Get next packet to send.
        let packet = self.poll_call_properties().or_else(|| self.poll_start_session_calls());
        let packet = packet.map(|(packet, span)| (packet, vec![span]));

        // Remove timed out & get next deadline. Nobody waits for the response
        // to `Properties`, so it doesn't need to wake up the protocol.
        if let PropertiesCall::Receiving { deadline, .. } = &self.properties_call
            && *deadline < time
        {
            self.properties_call = PropertiesCall::Idle;
        }
        let mut deadline = None;
        for queue in self.start_session_calls_receiving.values_mut() {
            while let Some(record) = queue.pop_front_if(|record| record.deadline < time) {
                let _ = record.sender.send(Err(Error::TimedOut));
            }
            if let Some(record) = queue.front() {
                deadline = min_deadline(deadline, Some(record.deadline))
            }
        }
        self.start_session_calls_receiving.retain(|_, queue| !queue.is_empty());

        // Decide action.
        if let Some(packet) = packet {
            Action::Send(vec![packet])
        } else if let Some(deadline) = deadline {
            Action::Sleep { until: deadline }
        } else {
            Action::None
        }
    }

    /// Returns the packet to send and the span of the `Properties` method call
    /// it carries.
    fn poll_call_properties(&mut self) -> Option<(Packet, Span)> {
        let PropertiesCall::Requested { span } = &self.properties_call else {
            return None;
        };
        let call = MethodCall {
            invoking_id: SESSION_MANAGER,
            method_id: PropertiesMethod::METHOD_ID,
            parameters: PropertiesMethod::Host { host_properties: Some(self.capabilities.clone()) },
            status: MethodStatus::Success,
        };
        let call = match call.to_tokens() {
            Ok(call) => call,
            Err(err) => {
                tracing::error!(parent: span, error = %err);
                self.properties_call = PropertiesCall::Idle;
                return None;
            }
        };
        // This call is pushed to the FRONT of the queue, NOT to the back.
        // This is fine, as SM methods are paired with the response by key,
        // not by order. This gives higher priority to property sync, so the
        // retrieved properties can be applied sooner.
        let sequence_number = self.sequence_number.fetch_add();
        let span = span.clone();
        self.properties_call = PropertiesCall::Sending { sequence_number, span: span.clone() };
        Some((packetize_one(SessionId::MANAGEMENT, sequence_number, call), span))
    }

    /// Returns a packet containing one or more `StartSession` method calls.
    /// The packet should be passed on for sending via IF-SEND.
    ///
    /// Currently only one method call is inside a packet, there is no batching.
    fn poll_start_session_calls(&mut self) -> Option<(Packet, Span)> {
        const MAX_METHOD_CALL_SIZE: usize =
            Properties::INITIAL.max_gross_packet_size.get() - PACKET_HEADER_LEN - SUB_PACKET_HEADER_LEN;

        loop {
            let StartSessionCallRecord { call: start_session, sender, span } =
                self.start_session_calls_enqueued.pop_front()?;

            let hsn = start_session.host_session_id;
            let maybe_tokens = MethodCall {
                invoking_id: SESSION_MANAGER,
                method_id: START_SESSION,
                parameters: start_session,
                status: MethodStatus::Success,
            }
            .to_tokens();

            let tokens = match maybe_tokens {
                Ok(tokens) => tokens,
                Err(err) => {
                    let _ = sender.send(Err(err.into()));
                    continue;
                }
            };

            if tokens.len() > MAX_METHOD_CALL_SIZE {
                let _ =
                    sender.send(Err(Error::MethodTooLarge { requested: tokens.len(), maximum: MAX_METHOD_CALL_SIZE }));
                continue;
            }

            let sequence_number = self.sequence_number.fetch_add();
            let record = StartSessionSendingRecord {
                sequence_number,
                sender,
                properties: self.properties.clone(),
                span: span.clone(),
            };
            self.start_session_calls_sending.entry(hsn).or_default().push_back(record);

            break Some((packetize_one(SessionId::MANAGEMENT, sequence_number, tokens), span));
        }
    }

    fn handle_sync_session(
        &mut self,
        actions: &mut Vec<StackAction>,
        sync_session: SyncSession,
        status: MethodStatus,
        source: &Span,
    ) {
        let session_id = SessionId { hsn: sync_session.host_session_id, tsn: sync_session.sp_session_id };
        if let Some(queue) = self.start_session_calls_receiving.get_mut(&sync_session.host_session_id) {
            if let Some(record) = queue.pop_front() {
                link_both_ways(source, &record.span);
                if status == MethodStatus::Success {
                    let _ = record.sender.send(Ok((sync_session, record.properties.clone())));
                    actions.push(StackAction::Spawn { session_id, properties: record.properties });
                } else {
                    let _ = record.sender.send(Err(status.into()));
                }
            }
            if queue.is_empty() {
                self.start_session_calls_receiving.remove(&session_id.hsn);
            }
        }
    }

    fn handle_close_session(actions: &mut Vec<StackAction>, close_session: CloseSession, status: MethodStatus) {
        if status == MethodStatus::Success {
            let session_id =
                SessionId { hsn: close_session.local_session_number, tsn: close_session.remote_session_number };
            actions.push(StackAction::NotifyAbort { session_id });
        }
    }

    fn handle_properties(&mut self, properties_method: PropertiesMethod, status: MethodStatus) {
        if status == MethodStatus::Success
            && let PropertiesMethod::TPer { properties, host_properties } = properties_method
        {
            // When we haven't initially sent our host properties to the TPer,
            // the TPer does not send the properties it will used when message
            // us. This also means the TPer is not aware of our capabilities,
            // and it will use the initial assumptions. If the TPer uses initial
            // assumptions, we shouldn't upgrade the connection properties, even
            // if the TPer indicated that it's capable of more.
            if host_properties.is_some() {
                self.properties = Properties::common(&self.capabilities, &properties);
            }
            let _ = self.properties_changed_tx.try_broadcast(PropertiesChanged {
                remote_properties: properties,
                connection_properties: self.properties.clone(),
            });
        }
    }

    fn flush(&mut self, error: Error) {
        self.received_tokens.clear();
        // The response to `Properties` may have been among the discarded tokens.
        if let PropertiesCall::Receiving { .. } = self.properties_call {
            self.properties_call = PropertiesCall::Idle;
        }
        for (_, queue) in self.start_session_calls_sending.drain() {
            for record in queue {
                let _ = record.sender.send(Err(error.clone()));
            }
        }
        for (_, queue) in self.start_session_calls_receiving.drain() {
            for record in queue {
                let _ = record.sender.send(Err(error.clone()));
            }
        }
    }

    /// The span of the method call that was sent the earliest among those
    /// awaiting a response.
    pub fn next_recv_span(&self) -> Option<(Instant, &Span)> {
        let properties = match &self.properties_call {
            PropertiesCall::Receiving { sent_at, span, .. } => Some((*sent_at, span)),
            _ => None,
        };
        self.start_session_calls_receiving
            .values()
            .filter_map(|queue| queue.front())
            .map(|record| (record.sent_at, &record.span))
            .chain(properties)
            .min_by_key(|(sent_at, _)| *sent_at)
    }

    pub fn capabilities(&self) -> &Properties {
        &self.capabilities
    }

    pub fn properties_changed(&self) -> async_broadcast::Receiver<PropertiesChanged> {
        self.properties_changed_rx.activate_cloned()
    }
}

#[derive(Debug, PartialEq, Eq)]
#[must_use]
pub enum StackAction {
    Spawn { session_id: SessionId, properties: Properties },
    NotifyAbort { session_id: SessionId },
}

#[derive(Debug, Clone)]
#[must_use]
pub enum Action {
    None,
    Sleep { until: Instant },
    Send(PacketBatch),
}

/// The state of the `Properties` method call that synchronizes the connection
/// properties with the TPer.
///
/// The response to `Properties` is processed whenever it arrives, regardless of
/// this state. Beyond requesting the method call, the state only serves to trace
/// it, and at most one call is tracked at a time.
#[derive(Debug)]
enum PropertiesCall {
    /// No sync requested or outstanding.
    Idle,
    /// Requested by the client, not yet packetized.
    Requested { span: Span },
    /// Packetized, waiting for IF-SEND to complete.
    Sending { sequence_number: SequenceNumber, span: Span },
    /// Sent, waiting for the TPer's `Properties` response.
    Receiving { sent_at: Instant, deadline: Instant, span: Span },
}

#[derive(Debug)]
pub struct StartSessionCallRecord {
    call: StartSession,
    sender: Sender<Result<(SyncSession, Properties), Error>>,
    span: Span,
}

#[derive(Debug)]
struct StartSessionSendingRecord {
    /// The sequence number of the packet in which the method is being sent.
    sequence_number: SequenceNumber,
    properties: Properties,
    sender: Sender<Result<(SyncSession, Properties), Error>>,
    /// The properties the host is using to send data to the TPer at the moment
    /// the `StartSession` was forwarded to IF-SEND.
    span: Span,
}

#[derive(Debug)]
struct StartSessionReceivingRecord {
    /// The time when the message was sent.
    sent_at: Instant,
    /// The time when the message times out.
    deadline: Instant,
    properties: Properties,
    sender: Sender<Result<(SyncSession, Properties), Error>>,
    span: Span,
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;
    use std::num::NonZero;

    use googletest::assert_that;
    use googletest::matchers::*;
    use oneshot::channel;
    use sed_packet::packet::SubPacket;
    use sed_packet::packet::SubPacketKind;
    use sed_packet::token_stream::ToTokens;
    use sed_spec::methods::Limit;
    use sed_spec::{
        methods::{MethodCall, MethodParam},
        preconfig::core::shared::invoking_id::SESSION_MANAGER,
    };

    use super::*;
    use crate::protocol::shared::tests::*;

    const SESSION_ID: SessionId = SessionId { hsn: 1, tsn: 2 };
    const TIMEOUT: Duration = Duration::from_secs(1);
    const HOST_PROPERTIES: Properties =
        Properties { max_methods: Limit::Limited(NonZero::new(10).unwrap()), ..Properties::INITIAL };
    const CONNECTION_PROPERTIES: Properties =
        Properties { max_methods: Limit::Limited(NonZero::new(5).unwrap()), ..Properties::INITIAL };
    const DEVICE_PROPERTIES: Properties =
        Properties { max_methods: Limit::Limited(NonZero::new(5).unwrap()), ..Properties::INITIAL };

    fn properties_device_call(host: bool) -> Vec<u8> {
        MethodCall {
            invoking_id: SESSION_MANAGER,
            method_id: PropertiesMethod::METHOD_ID,
            parameters: PropertiesMethod::TPer {
                properties: DEVICE_PROPERTIES,
                host_properties: host.then_some(HOST_PROPERTIES),
            },
            status: MethodStatus::Success,
        }
        .to_tokens()
        .unwrap()
    }

    #[test]
    fn start_session_completed_successfully() {
        let mut mgmt = Management::new(TIMEOUT, HOST_PROPERTIES);
        let time = Instant::now();
        let (sender, receiver) = channel();

        let start_session = start_session_call(SESSION_ID);
        let sync_session = sync_session_call(SESSION_ID, MethodStatus::Success);

        mgmt.handle_call_start_session(start_session.parameters.clone(), sender, Span::current());
        assert_that!(
            mgmt.poll_action(time),
            matches_pattern!(Action::Send(elements_are![(
                eq(&Packet {
                    tper_session_number: 0,
                    host_session_number: 0,
                    sequence_number: 1,
                    payload: vec![SubPacket {
                        kind: SubPacketKind::Data,
                        length: PhantomData,
                        payload: start_session.to_tokens().unwrap()
                    }],
                    ..Default::default()
                }),
                anything()
            )]))
        );

        mgmt.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        assert_that!(mgmt.poll_action(time), matches_pattern!(&Action::Sleep { until: eq(time + TIMEOUT) }));

        let stack_action = mgmt.handle_tokens(sync_session.to_tokens().unwrap(), &Span::none());
        assert_that!(
            stack_action,
            eq(&vec![StackAction::Spawn { session_id: SESSION_ID, properties: Properties::INITIAL }])
        );
        assert_that!(mgmt.poll_action(time), matches_pattern!(&Action::None));
        assert_that!(receiver.try_recv(), ok(ok(eq(&(sync_session.parameters, Properties::INITIAL)))));

        assert!(mgmt.start_session_calls_enqueued.is_empty());
        assert!(mgmt.start_session_calls_sending.is_empty());
        assert!(mgmt.start_session_calls_receiving.is_empty());
    }

    #[test]
    fn start_session_completed_with_error() {
        let mut mgmt = Management::new(TIMEOUT, HOST_PROPERTIES);
        let time = Instant::now();
        let (sender, receiver) = channel();

        let start_session = start_session_call(SESSION_ID);
        let sync_session = sync_session_call(SESSION_ID, MethodStatus::Fail);

        mgmt.handle_call_start_session(start_session.parameters.clone(), sender, Span::current());
        assert_that!(mgmt.poll_action(time), matches_pattern!(Action::Send(len(eq(1)))));

        mgmt.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        assert_that!(mgmt.poll_action(time), matches_pattern!(&Action::Sleep { until: eq(time + TIMEOUT) }));

        let stack_action = mgmt.handle_tokens(sync_session.to_tokens().unwrap(), &Span::none());
        assert_that!(stack_action, eq(&vec![]));
        assert_that!(mgmt.poll_action(time), matches_pattern!(&Action::None));
        assert_that!(receiver.try_recv(), ok(err(eq(&MethodStatus::Fail.into()))));

        assert!(mgmt.start_session_calls_enqueued.is_empty());
        assert!(mgmt.start_session_calls_sending.is_empty());
        assert!(mgmt.start_session_calls_receiving.is_empty());
    }

    #[test]
    fn start_session_timed_out() {
        let mut mgmt = Management::new(TIMEOUT, HOST_PROPERTIES);
        let time = Instant::now();
        let (sender, receiver) = channel();

        let start_session = start_session_call(SESSION_ID);

        mgmt.handle_call_start_session(start_session.parameters, sender, Span::current());
        assert_that!(mgmt.poll_action(time), matches_pattern!(Action::Send(len(eq(1)))));

        mgmt.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        assert_that!(mgmt.poll_action(time), matches_pattern!(&Action::Sleep { until: eq(time + TIMEOUT) }));
        assert_that!(mgmt.poll_action(time + 2 * TIMEOUT), matches_pattern!(&Action::None));

        assert_that!(receiver.try_recv(), ok(err(eq(&Error::TimedOut))));

        assert!(mgmt.start_session_calls_enqueued.is_empty());
        assert!(mgmt.start_session_calls_sending.is_empty());
        assert!(mgmt.start_session_calls_receiving.is_empty());
    }

    #[test]
    fn start_session_interface_send_failed() {
        let mut mgmt = Management::new(TIMEOUT, HOST_PROPERTIES);
        let time = Instant::now();
        let (sender, receiver) = channel();

        let start_session = start_session_call(SESSION_ID);

        mgmt.handle_call_start_session(start_session.parameters, sender, Span::current());
        assert_that!(mgmt.poll_action(time), matches_pattern!(Action::Send(len(eq(1)))));

        mgmt.handle_iface_send_done(time, SequenceNumber(1), Err(Error::NotSupported));
        assert_that!(mgmt.poll_action(time), matches_pattern!(&Action::None));

        assert_that!(receiver.try_recv(), ok(err(eq(&Error::NotSupported))));

        assert!(mgmt.start_session_calls_enqueued.is_empty());
        assert!(mgmt.start_session_calls_sending.is_empty());
        assert!(mgmt.start_session_calls_receiving.is_empty());
    }

    #[test]
    fn start_session_unexpected_tokens() {
        let mut mgmt = Management::new(TIMEOUT, HOST_PROPERTIES);

        let start_session = start_session_call(SESSION_ID);

        let stack_action = mgmt.handle_tokens(start_session.to_tokens().unwrap(), &Span::none());
        assert_that!(stack_action, eq(&vec![]));

        assert!(mgmt.start_session_calls_enqueued.is_empty());
        assert!(mgmt.start_session_calls_sending.is_empty());
        assert!(mgmt.start_session_calls_receiving.is_empty());
    }

    #[test]
    fn start_session_fragmented_tokens() {
        let mut mgmt = Management::new(TIMEOUT, HOST_PROPERTIES);
        let time = Instant::now();
        let (sender, receiver) = channel();

        let start_session = start_session_call(SESSION_ID);
        let sync_session = sync_session_call(SESSION_ID, MethodStatus::Success);

        let mut first_tokens = sync_session.to_tokens().unwrap();
        let second_tokens = first_tokens.split_off(2);

        mgmt.handle_call_start_session(start_session.parameters.clone(), sender, Span::current());
        assert_that!(
            mgmt.poll_action(time),
            matches_pattern!(Action::Send(elements_are![(
                eq(&Packet {
                    tper_session_number: 0,
                    host_session_number: 0,
                    sequence_number: 1,
                    payload: vec![SubPacket {
                        kind: SubPacketKind::Data,
                        length: PhantomData,
                        payload: start_session.to_tokens().unwrap(),
                    }],
                    ..Default::default()
                }),
                anything()
            )]))
        );

        mgmt.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        assert_that!(mgmt.poll_action(time), matches_pattern!(&Action::Sleep { until: eq(time + TIMEOUT) }));

        let stack_action = mgmt.handle_tokens(first_tokens, &Span::none());
        assert_that!(stack_action, eq(&vec![]));
        let stack_action = mgmt.handle_tokens(second_tokens, &Span::none());
        assert_that!(
            stack_action,
            eq(&vec![StackAction::Spawn { session_id: SESSION_ID, properties: Properties::INITIAL }])
        );
        assert_that!(mgmt.poll_action(time), matches_pattern!(&Action::None));
        assert_that!(receiver.try_recv(), ok(ok(eq(&(sync_session.parameters, Properties::INITIAL)))));

        assert!(mgmt.start_session_calls_enqueued.is_empty());
        assert!(mgmt.start_session_calls_sending.is_empty());
        assert!(mgmt.start_session_calls_receiving.is_empty());
    }

    #[test]
    fn start_session_invalid_tokens() {
        let mut mgmt = Management::new(TIMEOUT, HOST_PROPERTIES);
        let time = Instant::now();
        let (sender, receiver) = channel();
        let invalid_tokens = vec![0xFE, 34, 23, 7, 2, 3, 2];

        let start_session = start_session_call(SESSION_ID);

        mgmt.handle_call_start_session(start_session.parameters.clone(), sender, Span::current());
        assert_that!(
            mgmt.poll_action(time),
            matches_pattern!(Action::Send(elements_are![(
                eq(&Packet {
                    tper_session_number: 0,
                    host_session_number: 0,
                    sequence_number: 1,
                    payload: vec![SubPacket {
                        kind: SubPacketKind::Data,
                        length: PhantomData,
                        payload: start_session.to_tokens().unwrap(),
                    }],
                    ..Default::default()
                }),
                anything()
            )]))
        );

        mgmt.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        assert_that!(mgmt.poll_action(time), matches_pattern!(&Action::Sleep { until: eq(time + TIMEOUT) }));

        let stack_action = mgmt.handle_tokens(invalid_tokens, &Span::none());
        assert_that!(stack_action, eq(&vec![]));
        assert_that!(mgmt.poll_action(time), matches_pattern!(&Action::None));
        assert_that!(receiver.try_recv(), ok(err(pat!(&Error::TokenError(_)))));

        assert!(mgmt.start_session_calls_enqueued.is_empty());
        assert!(mgmt.start_session_calls_sending.is_empty());
        assert!(mgmt.start_session_calls_receiving.is_empty());
    }

    #[test]
    fn close_session_received() {
        let mut mgmt = Management::new(TIMEOUT, HOST_PROPERTIES);

        let stack_action = mgmt.handle_tokens(close_session_call(SESSION_ID), &Span::none());
        assert_that!(stack_action, eq(&vec![StackAction::NotifyAbort { session_id: SESSION_ID }]));
    }

    #[test]
    fn properties_received_without_host() {
        let mut mgmt = Management::new(TIMEOUT, HOST_PROPERTIES);
        let mut event = mgmt.properties_changed();

        let _ = mgmt.handle_tokens(properties_device_call(false), &Span::none());
        assert_that!(
            event.try_recv(),
            ok(&eq(&PropertiesChanged {
                remote_properties: DEVICE_PROPERTIES,
                connection_properties: Properties::INITIAL
            }))
        );
    }

    #[test]
    fn properties_received_with_host() {
        let mut mgmt = Management::new(TIMEOUT, HOST_PROPERTIES);
        let mut event = mgmt.properties_changed();

        let _ = mgmt.handle_tokens(properties_device_call(true), &Span::none());
        assert_that!(
            event.try_recv(),
            ok(&eq(&PropertiesChanged {
                remote_properties: DEVICE_PROPERTIES,
                connection_properties: CONNECTION_PROPERTIES
            }))
        );
    }
}
