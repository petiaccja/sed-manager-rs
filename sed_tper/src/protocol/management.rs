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
    token_stream::{FromTokens, ToTokens},
};
use sed_spec::{
    methods::{
        CloseSession, ExtractResult, MethodCall, MethodParam, MethodStatus, MgmtMethodCall, MgmtMethodCallParams,
        Properties, PropertiesMethod, SyncSession, extract_method,
    },
    preconfig::core::shared::invoking_id::SESSION_MANAGER,
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
    method_calls: VecDeque<MethodCallRecord>,
    /// `StartSession` methods that are queued for IF-SEND, keyed by HSN.
    start_session_calls_sending: HashMap<u32, VecDeque<MethodSendingRecord>>,
    start_session_calls_receiving: HashMap<u32, VecDeque<MethodReceivingRecord>>,
    received_tokens: VecDeque<u8>,
    properties_sync: PropertiesSync,
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
            method_calls: VecDeque::new(),
            start_session_calls_sending: HashMap::new(),
            start_session_calls_receiving: HashMap::new(),
            received_tokens: VecDeque::new(),
            properties_sync: PropertiesSync::Idle,
            properties_changed_tx,
            properties_changed_rx: properties_changed_rx.deactivate(),
        }
    }

    pub fn handle_method_call(&mut self, call: Vec<u8>, sender: Sender<Result<Vec<u8>, Error>>, span: Span) {
        self.method_calls.push_back(MethodCallRecord { call, sender, span });
    }

    /// Request synchronizing the connection properties with the TPer. The
    /// request is ignored while another one is already outstanding.
    pub fn handle_sync_properties(&mut self, span: Span) {
        if let PropertiesSync::Idle = self.properties_sync {
            self.properties_sync = PropertiesSync::Requested { span };
        }
    }

    pub fn handle_iface_send_done(&mut self, time: Instant, sn: SequenceNumber, result: Result<(), Error>) {
        for (hsn, queue) in &mut self.start_session_calls_sending {
            while let Some(record) = queue.pop_front_if(|record| record.sequence_number <= sn) {
                let deadline = time + self.timeout;
                match &result {
                    Ok(_) => {
                        let record =
                            MethodReceivingRecord { sent_at: time, deadline, sender: record.sender, span: record.span };
                        self.start_session_calls_receiving.entry(*hsn).or_default().push_back(record);
                    }
                    Err(err) => {
                        let _ = record.sender.send(Err(err.clone()));
                    }
                };
            }
        }
        self.start_session_calls_sending.retain(|_, queue| !queue.is_empty());

        self.properties_sync = match core::mem::replace(&mut self.properties_sync, PropertiesSync::Idle) {
            PropertiesSync::Sending { sequence_number, span } if sequence_number <= sn => match &result {
                Ok(_) => PropertiesSync::Receiving { sent_at: time, deadline: time + self.timeout, span },
                Err(_) => PropertiesSync::Idle,
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
                method_calls,
                start_session_calls_sending,
                start_session_calls_receiving,
                properties_changed_tx,
                properties_changed_rx,
                ..
            } = current;
            for MethodCallRecord { sender, .. } in method_calls {
                let _ = sender.send(Err(Error::Aborted));
            }
            for (_, queue) in start_session_calls_sending {
                for MethodSendingRecord { sender, .. } in queue {
                    let _ = sender.send(Err(Error::Aborted));
                }
            }
            for (_, queue) in start_session_calls_receiving {
                for MethodReceivingRecord { sender, .. } in queue {
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
                ExtractResult::Ok { value, tokens } => match value.params {
                    // The host should never receive a `StartSession`.
                    MgmtMethodCallParams::StartSession(_) => (),
                    MgmtMethodCallParams::SyncSession(sync_session) => {
                        self.handle_sync_session(&mut actions, sync_session, value.status, tokens, source)
                    }
                    MgmtMethodCallParams::CloseSession(close_session) => {
                        Self::handle_close_session(&mut actions, close_session, value.status);
                    }
                    MgmtMethodCallParams::Properties(properties_method) => {
                        if let PropertiesSync::Receiving { span, .. } = &self.properties_sync {
                            link_both_ways(source, span);
                            self.properties_sync = PropertiesSync::Idle;
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
        let packet = self.poll_sync_properties().or_else(|| self.poll_method_calls());
        let packet = packet.map(|(packet, span)| (packet, vec![span]));

        // Remove timed out & get next deadline. Nobody waits for the response
        // to `Properties`, so it doesn't need to wake up the protocol.
        if let PropertiesSync::Receiving { deadline, .. } = &self.properties_sync
            && *deadline < time
        {
            self.properties_sync = PropertiesSync::Idle;
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
    fn poll_sync_properties(&mut self) -> Option<(Packet, Span)> {
        let PropertiesSync::Requested { span } = &self.properties_sync else {
            return None;
        };
        let call = MethodCall {
            invoking_id: SESSION_MANAGER,
            method_id: PropertiesMethod::METHOD_ID,
            parameters: PropertiesMethod::Host { host_properties: Some(self.capabilities.clone()) },
            status: MethodStatus::Success,
        };
        let Ok(call) = call.to_tokens() else {
            // TODO: we should probably log this, even though it's not critical.
            self.properties_sync = PropertiesSync::Idle;
            return None;
        };
        // This call is pushed to the FRONT of the queue, NOT to the back.
        // This is fine, as SM methods are paired with the response by key,
        // not by order. This gives higher priority to property sync, so the
        // retrieved properties can be applied sooner.
        let sequence_number = self.sequence_number.fetch_add();
        let span = span.clone();
        self.properties_sync = PropertiesSync::Sending { sequence_number, span: span.clone() };
        Some((packetize_one(SessionId::MANAGEMENT, sequence_number, call), span))
    }

    /// Returns the packet to send and the span of the method calls it carries.
    /// Currently only one method call is inside a packet, there is no batching.
    fn poll_method_calls(&mut self) -> Option<(Packet, Span)> {
        const MAX_METHOD_CALL_SIZE: usize =
            Properties::INITIAL.max_gross_packet_size.get() - PACKET_HEADER_LEN - SUB_PACKET_HEADER_LEN;

        let MethodCallRecord { call, sender, span } = self.method_calls.pop_front()?;
        if call.len() > MAX_METHOD_CALL_SIZE {
            let _ = sender.send(Err(Error::MethodTooLarge { requested: call.len(), maximum: MAX_METHOD_CALL_SIZE }));
            return None;
        }

        match MgmtMethodCall::from_tokens(&call) {
            Ok(call_detok) => {
                let sequence_number = self.sequence_number.fetch_add();
                match call_detok.params {
                    MgmtMethodCallParams::StartSession(start_session) => {
                        let hsn = start_session.host_session_id;
                        let record = MethodSendingRecord { sequence_number, sender, span: span.clone() };
                        self.start_session_calls_sending.entry(hsn).or_default().push_back(record);
                        Some((packetize_one(SessionId::MANAGEMENT, sequence_number, call), span))
                    }
                    MgmtMethodCallParams::SyncSession(_) => {
                        // Method can only be sent by the device.
                        let _ = sender.send(Err(Error::MethodNotAllowed(SyncSession::METHOD_ID)));
                        None
                    }
                    MgmtMethodCallParams::CloseSession(_) => {
                        // Method can only be sent by the device.
                        let _ = sender.send(Err(Error::MethodNotAllowed(CloseSession::METHOD_ID)));
                        None
                    }
                    MgmtMethodCallParams::Properties(_) => {
                        // Could instruct the device to use capabilities that the protcol doesn't support.
                        let _ = sender.send(Err(Error::MethodNotAllowed(PropertiesMethod::METHOD_ID)));
                        None
                    }
                }
            }
            Err(err) => {
                let _ = sender.send(Err(err.into()));
                None
            }
        }
    }

    fn handle_sync_session(
        &mut self,
        actions: &mut Vec<StackAction>,
        sync_session: SyncSession,
        status: MethodStatus,
        tokens: Vec<u8>,
        source: &Span,
    ) {
        if let Some(queue) = self.start_session_calls_receiving.get_mut(&sync_session.host_session_id) {
            if let Some(record) = queue.pop_front() {
                link_both_ways(source, &record.span);
                if status == MethodStatus::Success {
                    let _ = record.sender.send(Ok(tokens));
                    let session_id = SessionId { hsn: sync_session.host_session_id, tsn: sync_session.sp_session_id };
                    // TODO: This is not entirely correct. The properties should be snapshot and saved when
                    // StartSession is sent out.
                    actions.push(StackAction::Spawn { session_id, properties: self.properties.clone() });
                } else {
                    let _ = record.sender.send(Err(status.into()));
                }
            }
            if queue.is_empty() {
                self.start_session_calls_receiving.remove(&sync_session.host_session_id);
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
        if let PropertiesSync::Receiving { .. } = self.properties_sync {
            self.properties_sync = PropertiesSync::Idle;
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
        let properties = match &self.properties_sync {
            PropertiesSync::Receiving { sent_at, span, .. } => Some((*sent_at, span)),
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
enum PropertiesSync {
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
pub struct MethodCallRecord {
    call: Vec<u8>,
    sender: Sender<Result<Vec<u8>, Error>>,
    span: Span,
}

#[derive(Debug)]
struct MethodSendingRecord {
    /// The sequence number of the packet in which the method is being sent.
    sequence_number: SequenceNumber,
    sender: Sender<Result<Vec<u8>, Error>>,
    span: Span,
}

#[derive(Debug)]
struct MethodReceivingRecord {
    /// The time when the message was sent.
    sent_at: Instant,
    /// The time when the message times out.
    deadline: Instant,
    sender: Sender<Result<Vec<u8>, Error>>,
    span: Span,
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;
    use std::num::NonZero;

    use googletest::assert_that;
    use googletest::matchers::*;
    use oneshot::channel;
    use rstest::rstest;
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

    fn properties_host_call() -> Vec<u8> {
        MethodCall {
            invoking_id: SESSION_MANAGER,
            method_id: PropertiesMethod::METHOD_ID,
            parameters: PropertiesMethod::Host { host_properties: None },
            status: MethodStatus::Success,
        }
        .to_tokens()
        .unwrap()
    }

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

    #[rstest]
    #[case::properties(properties_host_call())]
    #[case::sync_session(sync_session_call(SESSION_ID, MethodStatus::Success))]
    #[case::close_session(close_session_call(SESSION_ID,))]
    fn not_allowed_calls_intercepted(#[case] call: Vec<u8>) {
        let mut mgmt = Management::new(TIMEOUT, HOST_PROPERTIES);
        let time = Instant::now();
        let (sender, receiver) = channel();

        mgmt.handle_method_call(call, sender, Span::current());
        assert_that!(mgmt.poll_action(time), matches_pattern!(Action::None));
        assert_that!(receiver.try_recv(), ok(err(pat!(&Error::MethodNotAllowed { .. }))));
    }

    #[test]
    fn start_session_completed_successfully() {
        let mut mgmt = Management::new(TIMEOUT, HOST_PROPERTIES);
        let time = Instant::now();
        let (sender, receiver) = channel();

        mgmt.handle_method_call(start_session_call(SESSION_ID), sender, Span::current());
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
                        payload: start_session_call(SESSION_ID)
                    }],
                    ..Default::default()
                }),
                anything()
            )]))
        );

        mgmt.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        assert_that!(mgmt.poll_action(time), matches_pattern!(&Action::Sleep { until: eq(time + TIMEOUT) }));

        let stack_action = mgmt.handle_tokens(sync_session_call(SESSION_ID, MethodStatus::Success), &Span::none());
        assert_that!(
            stack_action,
            eq(&vec![StackAction::Spawn { session_id: SESSION_ID, properties: Properties::INITIAL }])
        );
        assert_that!(mgmt.poll_action(time), matches_pattern!(&Action::None));
        assert_that!(receiver.try_recv(), ok(ok(eq(&sync_session_call(SESSION_ID, MethodStatus::Success)))));

        assert!(mgmt.method_calls.is_empty());
        assert!(mgmt.start_session_calls_sending.is_empty());
        assert!(mgmt.start_session_calls_receiving.is_empty());
    }

    #[test]
    fn start_session_completed_with_error() {
        let mut mgmt = Management::new(TIMEOUT, HOST_PROPERTIES);
        let time = Instant::now();
        let (sender, receiver) = channel();

        mgmt.handle_method_call(start_session_call(SESSION_ID), sender, Span::current());
        assert_that!(mgmt.poll_action(time), matches_pattern!(Action::Send(len(eq(1)))));

        mgmt.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        assert_that!(mgmt.poll_action(time), matches_pattern!(&Action::Sleep { until: eq(time + TIMEOUT) }));

        let stack_action = mgmt.handle_tokens(sync_session_call(SESSION_ID, MethodStatus::Fail), &Span::none());
        assert_that!(stack_action, eq(&vec![]));
        assert_that!(mgmt.poll_action(time), matches_pattern!(&Action::None));
        assert_that!(receiver.try_recv(), ok(err(eq(&MethodStatus::Fail.into()))));

        assert!(mgmt.method_calls.is_empty());
        assert!(mgmt.start_session_calls_sending.is_empty());
        assert!(mgmt.start_session_calls_receiving.is_empty());
    }

    #[test]
    fn start_session_timed_out() {
        let mut mgmt = Management::new(TIMEOUT, HOST_PROPERTIES);
        let time = Instant::now();
        let (sender, receiver) = channel();

        mgmt.handle_method_call(start_session_call(SESSION_ID), sender, Span::current());
        assert_that!(mgmt.poll_action(time), matches_pattern!(Action::Send(len(eq(1)))));

        mgmt.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        assert_that!(mgmt.poll_action(time), matches_pattern!(&Action::Sleep { until: eq(time + TIMEOUT) }));
        assert_that!(mgmt.poll_action(time + 2 * TIMEOUT), matches_pattern!(&Action::None));

        assert_that!(receiver.try_recv(), ok(err(eq(&Error::TimedOut))));

        assert!(mgmt.method_calls.is_empty());
        assert!(mgmt.start_session_calls_sending.is_empty());
        assert!(mgmt.start_session_calls_receiving.is_empty());
    }

    #[test]
    fn start_session_interface_send_failed() {
        let mut mgmt = Management::new(TIMEOUT, HOST_PROPERTIES);
        let time = Instant::now();
        let (sender, receiver) = channel();

        mgmt.handle_method_call(start_session_call(SESSION_ID), sender, Span::current());
        assert_that!(mgmt.poll_action(time), matches_pattern!(Action::Send(len(eq(1)))));

        mgmt.handle_iface_send_done(time, SequenceNumber(1), Err(Error::NotSupported));
        assert_that!(mgmt.poll_action(time), matches_pattern!(&Action::None));

        assert_that!(receiver.try_recv(), ok(err(eq(&Error::NotSupported))));

        assert!(mgmt.method_calls.is_empty());
        assert!(mgmt.start_session_calls_sending.is_empty());
        assert!(mgmt.start_session_calls_receiving.is_empty());
    }

    #[test]
    fn start_session_unexpected_tokens() {
        let mut mgmt = Management::new(TIMEOUT, HOST_PROPERTIES);

        let stack_action = mgmt.handle_tokens(start_session_call(SESSION_ID), &Span::none());
        assert_that!(stack_action, eq(&vec![]));

        assert!(mgmt.method_calls.is_empty());
        assert!(mgmt.start_session_calls_sending.is_empty());
        assert!(mgmt.start_session_calls_receiving.is_empty());
    }

    #[test]
    fn start_session_fragmented_tokens() {
        let mut mgmt = Management::new(TIMEOUT, HOST_PROPERTIES);
        let time = Instant::now();
        let (sender, receiver) = channel();
        let mut first_tokens = sync_session_call(SESSION_ID, MethodStatus::Success);
        let second_tokens = first_tokens.split_off(2);

        mgmt.handle_method_call(start_session_call(SESSION_ID), sender, Span::current());
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
                        payload: start_session_call(SESSION_ID)
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
        assert_that!(receiver.try_recv(), ok(ok(eq(&sync_session_call(SESSION_ID, MethodStatus::Success)))));

        assert!(mgmt.method_calls.is_empty());
        assert!(mgmt.start_session_calls_sending.is_empty());
        assert!(mgmt.start_session_calls_receiving.is_empty());
    }

    #[test]
    fn start_session_invalid_tokens() {
        let mut mgmt = Management::new(TIMEOUT, HOST_PROPERTIES);
        let time = Instant::now();
        let (sender, receiver) = channel();
        let invalid_tokens = vec![0xFE, 34, 23, 7, 2, 3, 2];

        mgmt.handle_method_call(start_session_call(SESSION_ID), sender, Span::current());
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
                        payload: start_session_call(SESSION_ID)
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

        assert!(mgmt.method_calls.is_empty());
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
