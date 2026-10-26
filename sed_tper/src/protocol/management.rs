//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use oneshot::Sender;
use sed_packet::packet::Packet;
use sed_packet::session_id::SessionId;
use sed_packet::token_stream::ToTokens;
use sed_spec::methods::{
    CloseSession, ExtractResult, MethodCall, MethodParam, MethodStatus, MgmtMethodCall, MgmtMethodCallParams,
    Properties, PropertiesMethod, SessionProperties, StartSession, SyncSession, extract_method,
};
use sed_spec::preconfig::core::shared::invoking_id::SESSION_MANAGER;
use sed_spec::preconfig::core::shared::sm_method_id::START_SESSION;
use tracing::{Span, debug, info_span, warn};

use crate::Error;
use crate::protocol::sequence_number::SequenceNumber;
use crate::protocol::shared::{
    PacketAction, PropertiesChanged, StartedSession, link_both_ways, min_deadline, packetize_one,
};

#[derive(Debug)]
pub struct Management {
    timeout: Duration,
    capabilities: Properties,
    /// The initial assumptions about the communication properties, used until
    /// they are synced, and restored on a stack reset.
    initial_properties: SessionProperties,
    auto_sync_properties: bool,
    state: State,
    properties_changed_tx: async_broadcast::Sender<PropertiesChanged>,
    properties_changed_rx: async_broadcast::InactiveReceiver<PropertiesChanged>,
}

impl Management {
    pub fn new(
        timeout: Duration,
        capabilities: Properties,
        initial_properties: SessionProperties,
        auto_sync_properties: bool,
    ) -> Self {
        let (mut properties_changed_tx, properties_changed_rx) = async_broadcast::broadcast(1);
        properties_changed_tx.set_overflow(true);
        Self {
            timeout,
            capabilities,
            state: State::new(initial_properties.clone(), auto_sync_properties, Span::current()),
            initial_properties,
            auto_sync_properties,
            properties_changed_tx,
            properties_changed_rx: properties_changed_rx.deactivate(),
        }
    }

    pub fn handle_call_start_session(
        &mut self,
        call: StartSession,
        sender: Sender<Result<StartedSession, Error>>,
        span: Span,
    ) {
        self.state.start_session_calls_enqueued.push_back(StartSessionCallRecord { call, sender, span });
    }

    /// Request synchronizing the connection properties with the TPer. The
    /// request is ignored while another one is already outstanding.
    pub fn handle_call_properties(&mut self, span: Span) {
        if let PropertiesCall::Idle = self.state.properties_call {
            self.state.properties_call = PropertiesCall::Requested { span };
        } else {
            tracing::debug!(parent: &span, "ignored: another properties sync is outstanding");
        }
    }

    pub fn handle_iface_send_done(&mut self, time: Instant, sn: SequenceNumber, result: Result<(), Error>) {
        for (hsn, queue) in &mut self.state.start_session_calls_sending {
            while let Some(record) = queue.pop_front_if(|record| record.sequence_number <= sn) {
                let deadline = time + self.timeout;
                match &result {
                    Ok(_) => {
                        let record = StartSessionReceivingRecord {
                            sent_at: time,
                            deadline,
                            properties: self.state.properties.clone(),
                            sender: record.sender,
                            span: record.span,
                        };
                        self.state.start_session_calls_receiving.entry(*hsn).or_default().push_back(record);
                    }
                    Err(err) => {
                        let _ = record.sender.send(Err(err.clone()));
                    }
                };
            }
        }
        self.state.start_session_calls_sending.retain(|_, queue| !queue.is_empty());

        self.state.properties_call = match core::mem::replace(&mut self.state.properties_call, PropertiesCall::Idle) {
            PropertiesCall::Sending { sequence_number, span } if sequence_number <= sn => match &result {
                Ok(_) => PropertiesCall::Receiving { sent_at: time, deadline: time + self.timeout, span },
                Err(_) => PropertiesCall::Idle,
            },
            state => state,
        };
    }

    pub fn handle_reset(&mut self) {
        let state = State::new(self.initial_properties.clone(), self.auto_sync_properties, Span::current());
        core::mem::replace(&mut self.state, state).abort();

        // Inform that properties have been reset.
        let _ = self.properties_changed_tx.try_broadcast(PropertiesChanged {
            remote_capabilities: self.initial_properties.outbound.clone(),
            properties: self.state.properties.clone(),
        });
    }

    /// Process the tokens returned by the device.
    ///
    /// # Parameters
    ///
    /// - `tokens`: the tokens returned by the IF-RECV command.
    /// - `source`: the span of the IF-RECV command that returned the tokens.
    #[must_use]
    pub fn handle_tokens(&mut self, tokens: Vec<u8>, source: &Span) -> Vec<ManagementAction> {
        let mut actions = Vec::new();
        self.state.received_tokens.extend(tokens);
        loop {
            match extract_method::<MgmtMethodCall>(&mut self.state.received_tokens) {
                ExtractResult::Ok { value, .. } => {
                    let action = match value.params {
                        // The host should never receive a `StartSession`.
                        MgmtMethodCallParams::StartSession(_) => None,
                        MgmtMethodCallParams::SyncSession(sync_session) => {
                            self.handle_sync_session(sync_session, value.status, source)
                        }
                        MgmtMethodCallParams::CloseSession(close_session) => {
                            Self::handle_close_session(close_session, value.status)
                        }
                        MgmtMethodCallParams::Properties(properties_method) => {
                            if let PropertiesCall::Receiving { span, .. } = &self.state.properties_call {
                                link_both_ways(source, span);
                                self.state.properties_call = PropertiesCall::Idle;
                            }
                            self.handle_properties(properties_method, value.status, source)
                        }
                    };
                    actions.extend(action);
                }
                ExtractResult::EndOfSession => (),
                ExtractResult::NeedMoreTokens => break,
                ExtractResult::InvalidTokens(error) => {
                    self.handle_invalid_tokens(error.into());
                    break;
                }
            };
        }
        actions
    }

    pub fn poll_action(&mut self, time: Instant) -> PacketAction {
        // Remove timed out & get next deadline. The `Properties` deadline is
        // needed because pending `StartSession`s wait for it to complete.
        let mut deadline = None;
        if let PropertiesCall::Receiving { deadline: properties_deadline, .. } = &self.state.properties_call {
            if *properties_deadline < time {
                self.state.properties_call = PropertiesCall::Idle;
            } else {
                deadline = Some(*properties_deadline);
            }
        }
        for queue in self.state.start_session_calls_receiving.values_mut() {
            while let Some(record) = queue.pop_front_if(|record| record.deadline < time) {
                let _ = record.sender.send(Err(Error::TimedOut));
            }
            if let Some(record) = queue.front() {
                deadline = min_deadline(deadline, Some(record.deadline))
            }
        }
        self.state.start_session_calls_receiving.retain(|_, queue| !queue.is_empty());

        // Get next packet to send. Make sure either only `Properties` or
        // only `StartSession` methods are in flight at a time, or else
        // we cannot determine the active properties when the `StartSession` was
        // processed by the device.
        //
        // A requested `Properties` takes priority over enqueued `StartSession`s,
        // but it waits for the ones already in flight to complete.
        let properties_call = &self.state.properties_call;
        let packet = if !properties_call.is_requested() && !properties_call.is_in_flight() {
            self.poll_start_session_calls()
        } else if properties_call.is_requested() && !self.has_start_sessions_in_flight() {
            self.poll_call_properties()
        } else {
            None
        };
        let packet = packet.map(|(packet, span)| (packet, vec![span]));

        // Decide action.
        if let Some(packet) = packet {
            PacketAction::Send(vec![packet])
        } else if let Some(deadline) = deadline {
            PacketAction::Sleep { until: deadline }
        } else {
            PacketAction::None
        }
    }

    /// Returns the packet to send and the span of the `Properties` method call
    /// it carries.
    fn poll_call_properties(&mut self) -> Option<(Packet, Span)> {
        let PropertiesCall::Requested { span } = &self.state.properties_call else {
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
                self.state.properties_call = PropertiesCall::Idle;
                return None;
            }
        };
        let sequence_number = self.state.sequence_number.fetch_add();
        let span = span.clone();
        self.state.properties_call = PropertiesCall::Sending { sequence_number, span: span.clone() };
        Some((packetize_one(SessionId::MANAGEMENT, sequence_number, call), span))
    }

    /// Returns a packet containing one or more `StartSession` method calls.
    /// The packet should be passed on for sending via IF-SEND.
    ///
    /// Currently only one method call is inside a packet, there is no batching.
    fn poll_start_session_calls(&mut self) -> Option<(Packet, Span)> {
        loop {
            let StartSessionCallRecord { call: start_session, sender, span } =
                self.state.start_session_calls_enqueued.pop_front()?;

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

            // The negotiated limits are safe to use on the control session,
            // unlike the negotiated features, which the control session never
            // switches to.
            let max_method_len = self.state.properties.outbound.max_method_len();
            if tokens.len() > max_method_len {
                let _ = sender.send(Err(Error::MethodTooLarge { requested: tokens.len(), maximum: max_method_len }));
                continue;
            }

            let sequence_number = self.state.sequence_number.fetch_add();
            let record = StartSessionSendingRecord { sequence_number, sender, span: span.clone() };
            self.state.start_session_calls_sending.entry(hsn).or_default().push_back(record);

            break Some((packetize_one(SessionId::MANAGEMENT, sequence_number, tokens), span));
        }
    }

    fn handle_sync_session(
        &mut self,
        sync_session: SyncSession,
        status: MethodStatus,
        source: &Span,
    ) -> Option<ManagementAction> {
        let session_id = SessionId { hsn: sync_session.host_session_id, tsn: sync_session.sp_session_id };

        // Pop record (if any) and delete queue for HSN if empty.
        let maybe_record =
            self.state.start_session_calls_receiving.get_mut(&session_id.hsn).and_then(VecDeque::pop_front);
        if self.state.start_session_calls_receiving.get(&session_id.hsn).is_some_and(|queue| queue.is_empty()) {
            self.state.start_session_calls_receiving.remove(&session_id.hsn);
        }

        match (maybe_record, status) {
            (Some(record), MethodStatus::Success) => {
                let StartSessionReceivingRecord { properties, sender, span, .. } = record;

                link_both_ways(source, &span);

                let started_session = StartedSession { sync_session, properties: properties.clone() };

                if sender.send(Ok(started_session)).is_ok() {
                    Some(ManagementAction::Spawn { session_id, properties })
                } else {
                    Some(ManagementAction::SpawnOrphaned { session_id, properties })
                }
            }
            (Some(record), status) => {
                let _ = record.sender.send(Err(status.into()));
                None
            }
            (None, MethodStatus::Success) => {
                // We don't know the communication properties with which the
                // session was spawned, because we're not (or no longer)
                // tracking it. Without the properties, we cannot send an EOS,
                // because we don't know details like ACK/NAK. The only way out
                // is to reset the stack.
                warn!(parent: source, %session_id, "untracked successful `SyncSession` received, triggering stack reset");
                let span = info_span!(parent: source, "stack_reset", %session_id, reason = "untracked successful `SyncSession` received");
                Some(ManagementAction::StackReset { span })
            }
            (None, status) => {
                debug!(parent: source, %session_id, error = %status, "untracked failed `SyncSession` received");
                None
            }
        }
    }

    fn handle_close_session(close_session: CloseSession, status: MethodStatus) -> Option<ManagementAction> {
        if status == MethodStatus::Success {
            let session_id =
                SessionId { hsn: close_session.local_session_number, tsn: close_session.remote_session_number };
            Some(ManagementAction::NotifyAbort { session_id })
        } else {
            None
        }
    }

    fn handle_properties(
        &mut self,
        properties_method: PropertiesMethod,
        status: MethodStatus,
        source: &Span,
    ) -> Option<ManagementAction> {
        if status == MethodStatus::Success
            && let PropertiesMethod::TPer { properties: tper_capabilities, host_properties: inbound_properties } =
                properties_method
        {
            // If the TPer does not tell us what properties it uses for messaging
            // us, we must assume it uses the initial properties.
            let inbound_properties = inbound_properties.unwrap_or_else(|| self.initial_properties.inbound.clone());

            let outbound_properties = tper_capabilities
                .clone()
                .with_mutual_features(&inbound_properties) // Let the TPer voluntarily drop mutually supported features.
                .with_mutual_features(&self.capabilities) // Do not let the TPer unilaterally add features we don't support.
                .with_min_limits(&self.capabilities); // Do not use overly large buffers even when the TPer supports it.

            // The inbound and outbound features must match:
            // - Inbound: the TPer computed the mutually supported features.
            // - Outbound: we computed the mutually supported features.
            //
            // The TPer must also not use continued tokens when we don't support it.
            //
            // If the TPer violated the rules (i.e. firmware bug), we must
            // recover with a stack reset.
            if !outbound_properties.features_eq(&inbound_properties)
                || (inbound_properties.continued_tokens && !self.capabilities.continued_tokens)
            {
                warn!(
                    parent: source,
                    inbound.seq_numbers = inbound_properties.seq_numbers,
                    inbound.ack_nak = inbound_properties.ack_nak,
                    inbound.asynchronous = inbound_properties.asynchronous,
                    inbound.continued_tokens = inbound_properties.continued_tokens,
                    outbound.seq_numbers = outbound_properties.seq_numbers,
                    outbound.ack_nak = outbound_properties.ack_nak,
                    outbound.asynchronous = outbound_properties.asynchronous,
                    outbound.continued_tokens = outbound_properties.continued_tokens,
                    "the TPer wants to use an invalid/unsupported feature set for communication"
                );
                // Turn off auto-sync so that we don't get into a stack reset loop.
                self.auto_sync_properties = false;
                let span = info_span!(parent: source, "stack_reset", reason = "the TPer wants to use an invalid/unsupported feature set for communication");
                return Some(ManagementAction::StackReset { span });
            }

            // If the TPer wants to exceed our limits, we have to make sure not to
            // instruct the TPer to do so for methods like `Get` or `Random` where
            // we control the size of the data the TPer will send. If the TPer
            // still exceeds our limits, that may later trigger a stack reset,
            // but this is unlikely.
            let inbound_properties = inbound_properties.with_min_limits(&self.capabilities);

            self.state.properties = SessionProperties { outbound: outbound_properties, inbound: inbound_properties };

            let _ = self.properties_changed_tx.try_broadcast(PropertiesChanged {
                remote_capabilities: tper_capabilities,
                properties: self.state.properties.clone(),
            });
        }
        None
    }

    fn handle_invalid_tokens(&mut self, error: Error) {
        self.state.received_tokens.clear();
        // The response to `Properties` may have been among the discarded tokens.
        if self.state.properties_call.is_in_flight() {
            self.state.properties_call = PropertiesCall::Idle;
        }
        for (_, queue) in self.state.start_session_calls_sending.drain() {
            for record in queue {
                let _ = record.sender.send(Err(error.clone()));
            }
        }
        for (_, queue) in self.state.start_session_calls_receiving.drain() {
            for record in queue {
                let _ = record.sender.send(Err(error.clone()));
            }
        }
    }

    /// Whether any `StartSession` method has been packetized and is waiting
    /// for its IF-SEND or for its response. Enqueued methods don't count.
    fn has_start_sessions_in_flight(&self) -> bool {
        let State { start_session_calls_sending, start_session_calls_receiving, .. } = &self.state;
        !start_session_calls_sending.is_empty() || !start_session_calls_receiving.is_empty()
    }

    /// The span of the method call that was sent the earliest among those
    /// awaiting a response.
    pub fn next_recv_span(&self) -> Option<(Instant, &Span)> {
        let properties = match &self.state.properties_call {
            PropertiesCall::Receiving { sent_at, span, .. } => Some((*sent_at, span)),
            _ => None,
        };
        self.state
            .start_session_calls_receiving
            .values()
            .filter_map(|queue| queue.front())
            .map(|record| (record.sent_at, &record.span))
            .chain(properties)
            .min_by_key(|(sent_at, _)| *sent_at)
    }

    pub fn properties_changed(&self) -> async_broadcast::Receiver<PropertiesChanged> {
        self.properties_changed_rx.activate_cloned()
    }
}

/// The transient state of the [`Management`] session. This is discarded on a
/// stack reset.
#[derive(Debug)]
struct State {
    sequence_number: SequenceNumber,
    properties: SessionProperties,
    start_session_calls_enqueued: VecDeque<StartSessionCallRecord>,
    /// `StartSession` methods that have been submitted for IF-SEND, keyed by HSN.
    start_session_calls_sending: HashMap<u32, VecDeque<StartSessionSendingRecord>>,
    start_session_calls_receiving: HashMap<u32, VecDeque<StartSessionReceivingRecord>>,
    received_tokens: VecDeque<u8>,
    properties_call: PropertiesCall,
}

impl State {
    /// Create a new state. The `context` will be used for any packets that need
    /// to be sent right after startup.
    fn new(properties: SessionProperties, auto_sync_properties: bool, context: Span) -> Self {
        let properties_call = if auto_sync_properties {
            PropertiesCall::Requested { span: context }
        } else {
            PropertiesCall::Idle
        };

        Self {
            sequence_number: SequenceNumber::initial(),
            properties,
            start_session_calls_enqueued: VecDeque::new(),
            start_session_calls_sending: HashMap::new(),
            start_session_calls_receiving: HashMap::new(),
            received_tokens: VecDeque::new(),
            properties_call,
        }
    }

    /// Fail all outstanding calls with [`Error::Aborted`].
    ///
    fn abort(self) {
        // The destructuring is exhaustive on purpose: adding a field to `State`
        // requires deciding whether it needs finalizing on a stack reset.
        let Self {
            start_session_calls_enqueued,
            start_session_calls_sending,
            start_session_calls_receiving,
            sequence_number: _,
            properties: _,
            received_tokens: _,
            properties_call: _,
        } = self;
        for StartSessionCallRecord { sender, .. } in start_session_calls_enqueued {
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
    }
}

#[derive(Debug, PartialEq)]
#[must_use]
pub enum ManagementAction {
    /// Spawn a new session in response to a successful `SyncSession`.
    Spawn { session_id: SessionId, properties: SessionProperties },
    /// Spawn a new session in response to a successful `SyncSession` when the
    /// caller is no longer interested in the session (e.g. they dropped the
    /// channel or timed out). Since nobody will be sending anything on the
    /// session, it should be closed immediately.
    SpawnOrphaned { session_id: SessionId, properties: SessionProperties },
    /// Notify an active session that it has been aborted via a `CloseSession`.
    NotifyAbort { session_id: SessionId },
    /// The management session is unrecoverable and a stack reset must be
    /// performed.
    StackReset { span: Span },
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

impl PropertiesCall {
    /// A sync has been requested, but not yet packetized.
    fn is_requested(&self) -> bool {
        match self {
            PropertiesCall::Requested { .. } => true,
            PropertiesCall::Idle | PropertiesCall::Sending { .. } | PropertiesCall::Receiving { .. } => false,
        }
    }

    /// The `Properties` call has been packetized, and awaits its IF-SEND or
    /// its response.
    fn is_in_flight(&self) -> bool {
        match self {
            PropertiesCall::Sending { .. } | PropertiesCall::Receiving { .. } => true,
            PropertiesCall::Idle | PropertiesCall::Requested { .. } => false,
        }
    }
}

#[derive(Debug)]
pub struct StartSessionCallRecord {
    call: StartSession,
    sender: Sender<Result<StartedSession, Error>>,
    span: Span,
}

#[derive(Debug)]
struct StartSessionSendingRecord {
    /// The sequence number of the packet in which the method is being sent.
    sequence_number: SequenceNumber,
    sender: Sender<Result<StartedSession, Error>>,
    span: Span,
}

#[derive(Debug)]
struct StartSessionReceivingRecord {
    /// The time when the message was sent.
    sent_at: Instant,
    /// The time when the message times out.
    deadline: Instant,
    /// The properties in effect at the moment the `StartSession` was sent via
    /// IF-SEND.
    properties: SessionProperties,
    sender: Sender<Result<StartedSession, Error>>,
    span: Span,
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;
    use std::num::NonZero;

    use googletest::assert_that;
    use googletest::matchers::*;
    use oneshot::channel;
    use sed_packet::packet::{SubPacket, SubPacketKind};
    use sed_packet::token_stream::ToTokens;
    use sed_spec::methods::{Limit, MethodCall, MethodParam};
    use sed_spec::preconfig::core::shared::invoking_id::SESSION_MANAGER;

    use super::*;
    use crate::protocol::shared::tests::*;

    const SESSION_ID: SessionId = SessionId { hsn: 1, tsn: 2 };
    const TIMEOUT: Duration = Duration::from_secs(1);
    const HOST_CAPABILITIES: Properties =
        Properties { max_methods: Limit::Limited(NonZero::new(10).unwrap()), ..Properties::INITIAL };
    const TPER_CAPABILITIES: Properties =
        Properties { max_methods: Limit::Limited(NonZero::new(5).unwrap()), ..Properties::INITIAL };

    fn properties_device_call(host: bool) -> Vec<u8> {
        properties_response(TPER_CAPABILITIES, host.then_some(HOST_CAPABILITIES))
    }

    fn properties_response(properties: Properties, host_properties: Option<Properties>) -> Vec<u8> {
        MethodCall {
            invoking_id: SESSION_MANAGER,
            method_id: PropertiesMethod::METHOD_ID,
            parameters: PropertiesMethod::TPer { properties, host_properties },
            status: MethodStatus::Success,
        }
        .to_tokens()
        .unwrap()
    }

    #[test]
    fn start_session_completed_successfully() {
        let mut mgmt = Management::new(TIMEOUT, HOST_CAPABILITIES, SessionProperties::INITIAL, false);
        let time = Instant::now();
        let (sender, receiver) = channel();

        let start_session = start_session_call(SESSION_ID);
        let sync_session = sync_session_call(SESSION_ID, MethodStatus::Success);

        mgmt.handle_call_start_session(start_session.parameters.clone(), sender, Span::current());
        assert_that!(
            mgmt.poll_action(time),
            matches_pattern!(PacketAction::Send(elements_are![(
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
        assert_that!(mgmt.poll_action(time), matches_pattern!(&PacketAction::Sleep { until: eq(time + TIMEOUT) }));

        let stack_action = mgmt.handle_tokens(sync_session.to_tokens().unwrap(), &Span::none());
        assert_that!(
            stack_action,
            eq(&vec![ManagementAction::Spawn { session_id: SESSION_ID, properties: SessionProperties::INITIAL }])
        );
        assert_that!(mgmt.poll_action(time), matches_pattern!(&PacketAction::None));
        assert_that!(
            receiver.try_recv(),
            ok(ok(eq(&StartedSession {
                sync_session: sync_session.parameters,
                properties: SessionProperties::INITIAL
            })))
        );

        assert!(mgmt.state.start_session_calls_enqueued.is_empty());
        assert!(mgmt.state.start_session_calls_sending.is_empty());
        assert!(mgmt.state.start_session_calls_receiving.is_empty());
    }

    #[test]
    fn start_session_completed_with_error() {
        let mut mgmt = Management::new(TIMEOUT, HOST_CAPABILITIES, SessionProperties::INITIAL, false);
        let time = Instant::now();
        let (sender, receiver) = channel();

        let start_session = start_session_call(SESSION_ID);
        let sync_session = sync_session_call(SESSION_ID, MethodStatus::Fail);

        mgmt.handle_call_start_session(start_session.parameters.clone(), sender, Span::current());
        assert_that!(mgmt.poll_action(time), matches_pattern!(PacketAction::Send(len(eq(1)))));

        mgmt.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        assert_that!(mgmt.poll_action(time), matches_pattern!(&PacketAction::Sleep { until: eq(time + TIMEOUT) }));

        let stack_action = mgmt.handle_tokens(sync_session.to_tokens().unwrap(), &Span::none());
        assert_that!(stack_action, eq(&vec![]));
        assert_that!(mgmt.poll_action(time), matches_pattern!(&PacketAction::None));
        assert_that!(receiver.try_recv(), ok(err(eq(&MethodStatus::Fail.into()))));

        assert!(mgmt.state.start_session_calls_enqueued.is_empty());
        assert!(mgmt.state.start_session_calls_sending.is_empty());
        assert!(mgmt.state.start_session_calls_receiving.is_empty());
    }

    #[test]
    fn start_session_timed_out() {
        let mut mgmt = Management::new(TIMEOUT, HOST_CAPABILITIES, SessionProperties::INITIAL, false);
        let time = Instant::now();
        let (sender, receiver) = channel();

        let start_session = start_session_call(SESSION_ID);

        mgmt.handle_call_start_session(start_session.parameters, sender, Span::current());
        assert_that!(mgmt.poll_action(time), matches_pattern!(PacketAction::Send(len(eq(1)))));

        mgmt.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        assert_that!(mgmt.poll_action(time), matches_pattern!(&PacketAction::Sleep { until: eq(time + TIMEOUT) }));
        assert_that!(mgmt.poll_action(time + 2 * TIMEOUT), matches_pattern!(&PacketAction::None));

        assert_that!(receiver.try_recv(), ok(err(eq(&Error::TimedOut))));

        assert!(mgmt.state.start_session_calls_enqueued.is_empty());
        assert!(mgmt.state.start_session_calls_sending.is_empty());
        assert!(mgmt.state.start_session_calls_receiving.is_empty());
    }

    #[test]
    fn start_session_interface_send_failed() {
        let mut mgmt = Management::new(TIMEOUT, HOST_CAPABILITIES, SessionProperties::INITIAL, false);
        let time = Instant::now();
        let (sender, receiver) = channel();

        let start_session = start_session_call(SESSION_ID);

        mgmt.handle_call_start_session(start_session.parameters, sender, Span::current());
        assert_that!(mgmt.poll_action(time), matches_pattern!(PacketAction::Send(len(eq(1)))));

        mgmt.handle_iface_send_done(time, SequenceNumber(1), Err(Error::NotSupported));
        assert_that!(mgmt.poll_action(time), matches_pattern!(&PacketAction::None));

        assert_that!(receiver.try_recv(), ok(err(eq(&Error::NotSupported))));

        assert!(mgmt.state.start_session_calls_enqueued.is_empty());
        assert!(mgmt.state.start_session_calls_sending.is_empty());
        assert!(mgmt.state.start_session_calls_receiving.is_empty());
    }

    #[test]
    fn start_session_unexpected_tokens() {
        let mut mgmt = Management::new(TIMEOUT, HOST_CAPABILITIES, SessionProperties::INITIAL, false);

        let start_session = start_session_call(SESSION_ID);

        let stack_action = mgmt.handle_tokens(start_session.to_tokens().unwrap(), &Span::none());
        assert_that!(stack_action, eq(&vec![]));

        assert!(mgmt.state.start_session_calls_enqueued.is_empty());
        assert!(mgmt.state.start_session_calls_sending.is_empty());
        assert!(mgmt.state.start_session_calls_receiving.is_empty());
    }

    #[test]
    fn start_session_fragmented_tokens() {
        let mut mgmt = Management::new(TIMEOUT, HOST_CAPABILITIES, SessionProperties::INITIAL, false);
        let time = Instant::now();
        let (sender, receiver) = channel();

        let start_session = start_session_call(SESSION_ID);
        let sync_session = sync_session_call(SESSION_ID, MethodStatus::Success);

        let mut first_tokens = sync_session.to_tokens().unwrap();
        let second_tokens = first_tokens.split_off(2);

        mgmt.handle_call_start_session(start_session.parameters.clone(), sender, Span::current());
        assert_that!(
            mgmt.poll_action(time),
            matches_pattern!(PacketAction::Send(elements_are![(
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
        assert_that!(mgmt.poll_action(time), matches_pattern!(&PacketAction::Sleep { until: eq(time + TIMEOUT) }));

        let stack_action = mgmt.handle_tokens(first_tokens, &Span::none());
        assert_that!(stack_action, eq(&vec![]));
        let stack_action = mgmt.handle_tokens(second_tokens, &Span::none());
        assert_that!(
            stack_action,
            eq(&vec![ManagementAction::Spawn { session_id: SESSION_ID, properties: SessionProperties::INITIAL }])
        );
        assert_that!(mgmt.poll_action(time), matches_pattern!(&PacketAction::None));
        assert_that!(
            receiver.try_recv(),
            ok(ok(eq(&StartedSession {
                sync_session: sync_session.parameters,
                properties: SessionProperties::INITIAL
            })))
        );

        assert!(mgmt.state.start_session_calls_enqueued.is_empty());
        assert!(mgmt.state.start_session_calls_sending.is_empty());
        assert!(mgmt.state.start_session_calls_receiving.is_empty());
    }

    #[test]
    fn start_session_invalid_tokens() {
        let mut mgmt = Management::new(TIMEOUT, HOST_CAPABILITIES, SessionProperties::INITIAL, false);
        let time = Instant::now();
        let (sender, receiver) = channel();
        let invalid_tokens = vec![0xFE, 34, 23, 7, 2, 3, 2];

        let start_session = start_session_call(SESSION_ID);

        mgmt.handle_call_start_session(start_session.parameters.clone(), sender, Span::current());
        assert_that!(
            mgmt.poll_action(time),
            matches_pattern!(PacketAction::Send(elements_are![(
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
        assert_that!(mgmt.poll_action(time), matches_pattern!(&PacketAction::Sleep { until: eq(time + TIMEOUT) }));

        let stack_action = mgmt.handle_tokens(invalid_tokens, &Span::none());
        assert_that!(stack_action, eq(&vec![]));
        assert_that!(mgmt.poll_action(time), matches_pattern!(&PacketAction::None));
        assert_that!(receiver.try_recv(), ok(err(pat!(&Error::TokenError(_)))));

        assert!(mgmt.state.start_session_calls_enqueued.is_empty());
        assert!(mgmt.state.start_session_calls_sending.is_empty());
        assert!(mgmt.state.start_session_calls_receiving.is_empty());
    }

    #[test]
    fn start_session_properties_received_before_send() {
        let mut mgmt = Management::new(TIMEOUT, HOST_CAPABILITIES, SessionProperties::INITIAL, false);
        let time = Instant::now();
        let (sender, receiver) = channel();
        let negotiated = SessionProperties { outbound: TPER_CAPABILITIES, inbound: HOST_CAPABILITIES };

        let start_session = start_session_call(SESSION_ID);
        let sync_session = sync_session_call(SESSION_ID, MethodStatus::Success);

        // Both are queued, but only the `Properties` is sent. The `StartSession`
        // waits for its response.
        mgmt.handle_call_properties(Span::current());
        mgmt.handle_call_start_session(start_session.parameters.clone(), sender, Span::current());
        assert_that!(mgmt.poll_action(time), matches_pattern!(PacketAction::Send(len(eq(1)))));
        assert_that!(mgmt.poll_action(time), matches_pattern!(PacketAction::None));

        // The `Properties` response arrives, which releases the `StartSession`.
        mgmt.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        let stack_action =
            mgmt.handle_tokens(properties_response(TPER_CAPABILITIES, Some(HOST_CAPABILITIES)), &Span::none());
        assert_that!(stack_action, eq(&vec![]));
        assert_that!(mgmt.poll_action(time), matches_pattern!(PacketAction::Send(len(eq(1)))));
        mgmt.handle_iface_send_done(time, SequenceNumber(2), Ok(()));

        // The session uses the negotiated properties.
        let stack_action = mgmt.handle_tokens(sync_session.to_tokens().unwrap(), &Span::none());
        assert_that!(
            stack_action,
            eq(&vec![ManagementAction::Spawn { session_id: SESSION_ID, properties: negotiated.clone() }])
        );
        assert_that!(
            receiver.try_recv(),
            ok(ok(eq(&StartedSession { sync_session: sync_session.parameters, properties: negotiated })))
        );
    }

    #[test]
    fn start_session_properties_received_after_send() {
        let mut mgmt = Management::new(TIMEOUT, HOST_CAPABILITIES, SessionProperties::INITIAL, false);
        let time = Instant::now();
        let timed_out = time + TIMEOUT + Duration::from_millis(1);
        let (sender, receiver) = channel();
        let negotiated = SessionProperties { outbound: TPER_CAPABILITIES, inbound: HOST_CAPABILITIES };

        let start_session = start_session_call(SESSION_ID);
        let sync_session = sync_session_call(SESSION_ID, MethodStatus::Success);

        // The `Properties` is sent, and the `StartSession` waits for it.
        mgmt.handle_call_properties(Span::current());
        mgmt.handle_call_start_session(start_session.parameters.clone(), sender, Span::current());
        assert_that!(mgmt.poll_action(time), matches_pattern!(PacketAction::Send(len(eq(1)))));
        mgmt.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        assert_that!(mgmt.poll_action(time), matches_pattern!(PacketAction::Sleep { .. }));

        // The `Properties` times out, which releases the `StartSession`, even
        // though the TPer may still process the `Properties`.
        assert_that!(mgmt.poll_action(timed_out), matches_pattern!(PacketAction::Send(len(eq(1)))));
        mgmt.handle_iface_send_done(timed_out, SequenceNumber(2), Ok(()));

        // The late `Properties` response arrives after the `StartSession` was
        // sent. It's applied, but it doesn't affect the session being started.
        let stack_action = mgmt.handle_tokens(properties_device_call(true), &Span::none());
        assert_that!(stack_action, eq(&vec![]));
        assert_that!(mgmt.state.properties, eq(&negotiated));

        // The TPer may have processed the `Properties` after the `StartSession`,
        // so the session must stay on the properties confirmed at sending.
        let stack_action = mgmt.handle_tokens(sync_session.to_tokens().unwrap(), &Span::none());
        assert_that!(
            stack_action,
            eq(&vec![ManagementAction::Spawn { session_id: SESSION_ID, properties: SessionProperties::INITIAL }])
        );
        assert_that!(
            receiver.try_recv(),
            ok(ok(eq(&StartedSession {
                sync_session: sync_session.parameters,
                properties: SessionProperties::INITIAL
            })))
        );
    }

    #[test]
    fn prioritize_properties_over_start_session() {
        let mut mgmt = Management::new(TIMEOUT, HOST_CAPABILITIES, SessionProperties::INITIAL, false);
        let time = Instant::now();
        let (sender, receiver) = channel();
        let negotiated = SessionProperties { outbound: TPER_CAPABILITIES, inbound: HOST_CAPABILITIES };

        let start_session = start_session_call(SESSION_ID);
        let sync_session = sync_session_call(SESSION_ID, MethodStatus::Success);

        // Both are queued, the `StartSession` first, but the `Properties` is
        // sent first.
        mgmt.handle_call_start_session(start_session.parameters.clone(), sender, Span::current());
        mgmt.handle_call_properties(Span::current());
        assert_that!(mgmt.poll_action(time), matches_pattern!(PacketAction::Send(len(eq(1)))));
        assert_that!(mgmt.poll_action(time), matches_pattern!(PacketAction::None));

        // While the `Properties` awaits its response, the protocol sleeps until
        // the `Properties` times out.
        mgmt.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        assert_that!(mgmt.poll_action(time), matches_pattern!(PacketAction::Sleep { .. }));

        // The `Properties` response arrives, which releases the `StartSession`.
        let stack_action = mgmt.handle_tokens(properties_device_call(true), &Span::none());
        assert_that!(stack_action, eq(&vec![]));
        assert_that!(mgmt.poll_action(time), matches_pattern!(PacketAction::Send(len(eq(1)))));
        mgmt.handle_iface_send_done(time, SequenceNumber(2), Ok(()));

        // The session uses the negotiated properties.
        let stack_action = mgmt.handle_tokens(sync_session.to_tokens().unwrap(), &Span::none());
        assert_that!(
            stack_action,
            eq(&vec![ManagementAction::Spawn { session_id: SESSION_ID, properties: negotiated.clone() }])
        );
        assert_that!(
            receiver.try_recv(),
            ok(ok(eq(&StartedSession { sync_session: sync_session.parameters, properties: negotiated })))
        );
    }

    #[test]
    fn properties_held_while_start_session_in_flight() {
        let mut mgmt = Management::new(TIMEOUT, HOST_CAPABILITIES, SessionProperties::INITIAL, false);
        let time = Instant::now();
        let (sender, receiver) = channel();
        let negotiated = SessionProperties { outbound: TPER_CAPABILITIES, inbound: HOST_CAPABILITIES };

        let start_session = start_session_call(SESSION_ID);
        let sync_session = sync_session_call(SESSION_ID, MethodStatus::Success);

        // The `StartSession` is sent first, so a `Properties` requested
        // afterwards must wait for the `StartSession` to complete.
        mgmt.handle_call_start_session(start_session.parameters.clone(), sender, Span::current());
        assert_that!(mgmt.poll_action(time), matches_pattern!(PacketAction::Send(len(eq(1)))));
        mgmt.handle_call_properties(Span::current());
        assert_that!(mgmt.poll_action(time), matches_pattern!(PacketAction::None));
        mgmt.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        assert_that!(mgmt.poll_action(time), matches_pattern!(PacketAction::Sleep { .. }));

        // The session uses the properties from before the `Properties` call,
        // since the TPer hasn't received it yet.
        let stack_action = mgmt.handle_tokens(sync_session.to_tokens().unwrap(), &Span::none());
        assert_that!(
            stack_action,
            eq(&vec![ManagementAction::Spawn { session_id: SESSION_ID, properties: SessionProperties::INITIAL }])
        );
        assert_that!(
            receiver.try_recv(),
            ok(ok(eq(&StartedSession {
                sync_session: sync_session.parameters,
                properties: SessionProperties::INITIAL
            })))
        );

        // The `StartSession` completed, which releases the `Properties`.
        assert_that!(mgmt.poll_action(time), matches_pattern!(PacketAction::Send(len(eq(1)))));
        mgmt.handle_iface_send_done(time, SequenceNumber(2), Ok(()));
        let stack_action = mgmt.handle_tokens(properties_device_call(true), &Span::none());
        assert_that!(stack_action, eq(&vec![]));
        assert_that!(mgmt.state.properties, eq(&negotiated));
    }

    #[test]
    fn properties_timed_out() {
        let mut mgmt = Management::new(TIMEOUT, HOST_CAPABILITIES, SessionProperties::INITIAL, false);
        let time = Instant::now();
        let timed_out = time + TIMEOUT + Duration::from_millis(1);

        mgmt.handle_call_properties(Span::current());
        assert_that!(mgmt.poll_action(time), matches_pattern!(PacketAction::Send(len(eq(1)))));
        mgmt.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        assert_that!(mgmt.poll_action(time), matches_pattern!(PacketAction::Sleep { .. }));

        // The expired deadline must not be reported for sleeping.
        assert_that!(mgmt.poll_action(timed_out), matches_pattern!(PacketAction::None));
        assert_that!(mgmt.state.properties_call, matches_pattern!(PropertiesCall::Idle));
    }

    #[test]
    fn sync_session_after_timeout() {
        let mut mgmt = Management::new(TIMEOUT, HOST_CAPABILITIES, SessionProperties::INITIAL, false);
        let time = Instant::now();
        let (sender, receiver) = channel();

        let start_session = start_session_call(SESSION_ID);
        let sync_session = sync_session_call(SESSION_ID, MethodStatus::Success);

        mgmt.handle_call_start_session(start_session.parameters, sender, Span::current());
        assert_that!(mgmt.poll_action(time), matches_pattern!(PacketAction::Send(len(eq(1)))));
        mgmt.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        assert_that!(mgmt.poll_action(time + 2 * TIMEOUT), matches_pattern!(&PacketAction::None));
        assert_that!(receiver.try_recv(), ok(err(eq(&Error::TimedOut))));

        let stack_action = mgmt.handle_tokens(sync_session.to_tokens().unwrap(), &Span::none());
        assert_that!(stack_action, elements_are!(pat!(ManagementAction::StackReset { .. })));
    }

    #[test]
    fn sync_session_unexpected() {
        let mut mgmt = Management::new(TIMEOUT, HOST_CAPABILITIES, SessionProperties::INITIAL, false);

        let sync_session = sync_session_call(SESSION_ID, MethodStatus::Success);

        let stack_action = mgmt.handle_tokens(sync_session.to_tokens().unwrap(), &Span::none());
        assert_that!(stack_action, elements_are!(pat!(ManagementAction::StackReset { .. })));
    }

    #[test]
    fn sync_session_unexpected_failed() {
        let mut mgmt = Management::new(TIMEOUT, HOST_CAPABILITIES, SessionProperties::INITIAL, false);

        let sync_session = sync_session_call(SESSION_ID, MethodStatus::Fail);

        let stack_action = mgmt.handle_tokens(sync_session.to_tokens().unwrap(), &Span::none());
        assert_that!(stack_action, is_empty());
    }

    #[test]
    fn sync_session_receiver_dropped() {
        let mut mgmt = Management::new(TIMEOUT, HOST_CAPABILITIES, SessionProperties::INITIAL, false);
        let time = Instant::now();
        let (sender, receiver) = channel();

        let start_session = start_session_call(SESSION_ID);
        let sync_session = sync_session_call(SESSION_ID, MethodStatus::Success);

        mgmt.handle_call_start_session(start_session.parameters, sender, Span::current());
        assert_that!(mgmt.poll_action(time), matches_pattern!(PacketAction::Send(len(eq(1)))));
        mgmt.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        drop(receiver);

        let stack_action = mgmt.handle_tokens(sync_session.to_tokens().unwrap(), &Span::none());
        assert_that!(
            stack_action,
            elements_are!(eq(&ManagementAction::SpawnOrphaned {
                session_id: SESSION_ID,
                properties: SessionProperties::INITIAL,
            }))
        );
        assert!(mgmt.state.start_session_calls_receiving.is_empty());
    }

    #[test]
    fn close_session_received() {
        let mut mgmt = Management::new(TIMEOUT, HOST_CAPABILITIES, SessionProperties::INITIAL, false);

        let stack_action = mgmt.handle_tokens(close_session_call(SESSION_ID), &Span::none());
        assert_that!(stack_action, eq(&vec![ManagementAction::NotifyAbort { session_id: SESSION_ID }]));
    }

    #[test]
    fn properties_received_without_host() {
        let mut mgmt = Management::new(TIMEOUT, HOST_CAPABILITIES, SessionProperties::INITIAL, false);
        let mut event = mgmt.properties_changed();

        let _ = mgmt.handle_tokens(properties_device_call(false), &Span::none());
        assert_that!(
            event.try_recv(),
            ok(&eq(&PropertiesChanged {
                remote_capabilities: TPER_CAPABILITIES,
                properties: SessionProperties { outbound: TPER_CAPABILITIES, inbound: Properties::INITIAL },
            }))
        );
    }

    #[test]
    fn properties_received_with_host() {
        let mut mgmt = Management::new(TIMEOUT, HOST_CAPABILITIES, SessionProperties::INITIAL, false);
        let mut event = mgmt.properties_changed();

        let _ = mgmt.handle_tokens(properties_device_call(true), &Span::none());
        assert_that!(
            event.try_recv(),
            ok(&eq(&PropertiesChanged {
                remote_capabilities: TPER_CAPABILITIES,
                properties: SessionProperties { outbound: TPER_CAPABILITIES, inbound: HOST_CAPABILITIES },
            }))
        );
    }

    #[test]
    fn properties_changed_lagging_receiver_gets_latest() {
        let mut mgmt = Management::new(TIMEOUT, HOST_CAPABILITIES, SessionProperties::INITIAL, false);
        let mut event = mgmt.properties_changed();

        mgmt.handle_reset();
        let _ = mgmt.handle_tokens(properties_device_call(true), &Span::none());

        // The receiver is told that it missed an event, and then sees the
        // latest value only.
        assert_that!(event.try_recv(), err(eq(&async_broadcast::TryRecvError::Overflowed(1))));
        assert_that!(
            event.try_recv(),
            ok(&eq(&PropertiesChanged {
                remote_capabilities: TPER_CAPABILITIES,
                properties: SessionProperties { outbound: TPER_CAPABILITIES, inbound: HOST_CAPABILITIES },
            }))
        );
        assert_that!(event.try_recv(), err(eq(&async_broadcast::TryRecvError::Empty)));
    }

    #[test]
    fn properties_changed_idle_receiver_does_not_block() {
        let mut mgmt = Management::new(TIMEOUT, HOST_CAPABILITIES, SessionProperties::INITIAL, false);
        let _idle = mgmt.properties_changed();
        let mut event = mgmt.properties_changed();

        mgmt.handle_reset();
        assert_that!(
            event.try_recv(),
            ok(&eq(&PropertiesChanged {
                remote_capabilities: Properties::INITIAL,
                properties: SessionProperties::INITIAL,
            }))
        );

        let _ = mgmt.handle_tokens(properties_device_call(true), &Span::none());
        assert_that!(
            event.try_recv(),
            ok(&eq(&PropertiesChanged {
                remote_capabilities: TPER_CAPABILITIES,
                properties: SessionProperties { outbound: TPER_CAPABILITIES, inbound: HOST_CAPABILITIES },
            }))
        );
    }

    #[test]
    fn properties_not_auto_synced() {
        let mut mgmt = Management::new(TIMEOUT, HOST_CAPABILITIES, SessionProperties::INITIAL, false);
        let time = Instant::now();

        let action = mgmt.poll_action(time);
        assert_that!(action, pat!(PacketAction::None));
    }

    #[test]
    fn properties_auto_synced() {
        let mut mgmt = Management::new(TIMEOUT, HOST_CAPABILITIES, SessionProperties::INITIAL, true);
        let time = Instant::now();

        let action = mgmt.poll_action(time);
        assert_that!(action, pat!(PacketAction::Send(len(eq(1)))));
    }

    #[test]
    fn properties_limits_capped_by_host() {
        let mut mgmt = Management::new(TIMEOUT, HOST_CAPABILITIES, SessionProperties::INITIAL, false);
        let mut event = mgmt.properties_changed();

        // Both the TPer's capabilities and the echo exceed the host's
        // `MaxMethods` of 10.
        let exceeding = Properties { max_methods: Limit::Limited(NonZero::new(20).unwrap()), ..Properties::INITIAL };
        let _ = mgmt.handle_tokens(properties_response(exceeding.clone(), Some(exceeding.clone())), &Span::none());
        assert_that!(
            event.try_recv(),
            ok(&eq(&PropertiesChanged {
                remote_capabilities: exceeding,
                properties: SessionProperties { outbound: HOST_CAPABILITIES, inbound: HOST_CAPABILITIES },
            }))
        );
    }

    /// Returns a [`Properties`] with the given features (sequence numbers,
    /// ACK/NAK) and initial values otherwise.
    fn features((seq_numbers, ack_nak): (bool, bool)) -> Properties {
        Properties { seq_numbers, ack_nak, ..Properties::INITIAL }
    }

    #[rstest::rstest]
    #[case::none((false, false), (false, false), (false, false), (false, false))]
    #[case::mutual((true, true), (true, true), (true, true), (true, true))]
    #[case::tper_downgrades_echo((true, true), (true, true), (true, false), (true, false))]
    #[case::echo_omits_features((true, true), (true, true), (false, false), (false, false))]
    fn properties_mutual_features(
        #[case] host: (bool, bool),
        #[case] tper: (bool, bool),
        #[case] echo: (bool, bool),
        #[case] expected: (bool, bool),
    ) {
        let mut mgmt = Management::new(TIMEOUT, features(host), SessionProperties::INITIAL, false);
        let mut event = mgmt.properties_changed();

        let actions = mgmt.handle_tokens(properties_response(features(tper), Some(features(echo))), &Span::none());
        assert_that!(actions, is_empty());
        let event = event.try_recv().unwrap();
        let outbound = &event.properties.outbound;
        let inbound = &event.properties.inbound;
        assert_that!((outbound.seq_numbers, outbound.ack_nak), eq(expected));
        assert_that!((inbound.seq_numbers, inbound.ack_nak), eq(expected));
    }

    #[rstest::rstest]
    #[case::echo_exceeds_host((false, false), (true, true), (true, true))]
    #[case::echo_exceeds_tper((true, true), (true, false), (true, true))]
    #[case::ack_nak_without_seq_numbers((true, true), (true, true), (false, true))]
    fn properties_feature_mismatch_resets_stack(
        #[case] host: (bool, bool),
        #[case] tper: (bool, bool),
        #[case] echo: (bool, bool),
    ) {
        let mut mgmt = Management::new(TIMEOUT, features(host), SessionProperties::INITIAL, true);
        let mut event = mgmt.properties_changed();

        let actions = mgmt.handle_tokens(properties_response(features(tper), Some(features(echo))), &Span::none());
        assert_that!(actions, elements_are![pat!(ManagementAction::StackReset { .. })]);
        assert!(!mgmt.auto_sync_properties);

        // The mismatched properties are not applied.
        assert_that!(event.try_recv(), err(eq(&async_broadcast::TryRecvError::Empty)));
        assert_that!(mgmt.state.properties, eq(&SessionProperties::INITIAL));
    }

    #[rstest::rstest]
    #[case::not_used(false, false, false)]
    #[case::supported_and_used(true, true, false)]
    #[case::supported_not_used(true, false, false)]
    #[case::unsupported_but_used(false, true, true)]
    fn properties_continued_tokens(#[case] host: bool, #[case] echo: bool, #[case] reset: bool) {
        let host = Properties { continued_tokens: host, ..Properties::INITIAL };
        // The TPer accepting continued tokens from us has no bearing on whether
        // it may send them to us.
        let tper = Properties { continued_tokens: true, ..Properties::INITIAL };
        let echo = Properties { continued_tokens: echo, ..Properties::INITIAL };
        let mut mgmt = Management::new(TIMEOUT, host, SessionProperties::INITIAL, true);
        let mut event = mgmt.properties_changed();

        let actions = mgmt.handle_tokens(properties_response(tper, Some(echo.clone())), &Span::none());
        if reset {
            assert_that!(actions, elements_are![pat!(ManagementAction::StackReset { .. })]);
            assert!(!mgmt.auto_sync_properties);
            assert_that!(event.try_recv(), err(eq(&async_broadcast::TryRecvError::Empty)));
        } else {
            assert_that!(actions, is_empty());
            assert!(mgmt.auto_sync_properties);
            let event = event.try_recv().unwrap();
            assert_that!(event.properties.inbound.continued_tokens, eq(echo.continued_tokens));
        }
    }

    #[test]
    fn reset_restores_initial_properties() {
        let initial =
            SessionProperties { outbound: Properties::INITIAL_OPAL_FAMILY, inbound: Properties::INITIAL_OPAL_FAMILY };
        let mut mgmt = Management::new(TIMEOUT, Properties::INITIAL_OPAL_FAMILY, initial.clone(), false);
        assert_that!(mgmt.state.properties, eq(&initial));

        // The properties are synced, then reset.
        let _ = mgmt.handle_tokens(properties_device_call(true), &Span::none());
        assert_that!(mgmt.state.properties, not(eq(&initial)));
        let mut event = mgmt.properties_changed();
        mgmt.handle_reset();

        // Both the state and the event go back to the initial assumptions, not
        // to the core specification's.
        assert_that!(mgmt.state.properties, eq(&initial));
        let event = event.try_recv().unwrap();
        assert_that!(event.remote_capabilities, eq(&Properties::INITIAL_OPAL_FAMILY));
        assert_that!(event.properties, eq(&initial));
    }

    #[test]
    fn properties_without_echo_use_initial_inbound() {
        let initial =
            SessionProperties { outbound: Properties::INITIAL_OPAL_FAMILY, inbound: Properties::INITIAL_OPAL_FAMILY };
        let mut mgmt = Management::new(TIMEOUT, Properties::INITIAL_OPAL_FAMILY, initial, false);

        // Without the echo, the TPer keeps using its initial assumptions about
        // the host, which are the SSC's, not the core specification's.
        let stack_action = mgmt.handle_tokens(properties_response(TPER_CAPABILITIES, None), &Span::none());
        assert_that!(stack_action, eq(&vec![]));
        assert_that!(mgmt.state.properties.inbound, eq(&Properties::INITIAL_OPAL_FAMILY));
    }

    #[test]
    fn reset_resyncs_properties_with_auto_sync() {
        let mut mgmt = Management::new(TIMEOUT, HOST_CAPABILITIES, SessionProperties::INITIAL, true);
        let time = Instant::now();

        mgmt.handle_reset();
        let action = mgmt.poll_action(time);
        assert_that!(action, pat!(PacketAction::Send(len(eq(1)))));
    }

    #[test]
    fn reset_after_feature_mismatch_does_not_resync() {
        let mut mgmt = Management::new(TIMEOUT, HOST_CAPABILITIES, SessionProperties::INITIAL, true);
        let time = Instant::now();

        let mismatch = features((true, true));
        let actions = mgmt.handle_tokens(properties_response(mismatch.clone(), Some(mismatch)), &Span::none());
        assert_that!(actions, elements_are![pat!(ManagementAction::StackReset { .. })]);

        // Another sync would get the same echo and trigger another reset.
        mgmt.handle_reset();
        let action = mgmt.poll_action(time);
        assert_that!(action, pat!(PacketAction::None));
    }
}
