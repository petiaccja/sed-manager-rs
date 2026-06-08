use std::{
    collections::{HashMap, VecDeque},
    time::{Duration, Instant},
};

use oneshot::Sender;
use sed_packet::{
    packet::{PACKET_HEADER_LEN, Packet, SUB_PACKET_HEADER_LEN},
    session_id::SessionId,
    token::FromTokens,
};
use sed_spec::methods::{
    CloseSession, ExtractResult, MethodStatus, MgmtMethodCall, MgmtMethodCallParams, Properties, PropertiesMethod,
    SyncSession, extract_method,
};

use crate::{
    Error,
    protocol::{
        sequence_number::SequenceNumber,
        shared::{min_deadline, packetize_one},
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
}

impl Management {
    pub fn new(timeout: Duration, capabilities: Properties) -> Self {
        Self {
            sequence_number: SequenceNumber::initial(),
            timeout,
            capabilities,
            properties: Properties::INITIAL,
            method_calls: VecDeque::new(),
            start_session_calls_sending: HashMap::new(),
            start_session_calls_receiving: HashMap::new(),
            received_tokens: VecDeque::new(),
        }
    }

    pub fn handle_method_call(&mut self, call: Vec<u8>, sender: Sender<Result<Vec<u8>, Error>>) {
        self.method_calls.push_back(MethodCallRecord { call, sender });
    }

    pub fn handle_iface_send_done(&mut self, time: Instant, sn: SequenceNumber, result: Result<(), Error>) {
        for (hsn, queue) in &mut self.start_session_calls_sending {
            while let Some(record) = queue.pop_front_if(|record| record.sequence_number <= sn) {
                let deadline = time + self.timeout;
                match &result {
                    Ok(_) => {
                        let record = MethodReceivingRecord { deadline, sender: record.sender };
                        self.start_session_calls_receiving.entry(*hsn).or_default().push_back(record);
                    }
                    Err(err) => {
                        let _ = record.sender.send(Err(err.clone()));
                    }
                };
            }
        }
        self.start_session_calls_sending.retain(|_, queue| !queue.is_empty());
    }

    pub fn handle_reset(&mut self) {
        for MethodCallRecord { sender, .. } in self.method_calls.drain(..) {
            let _ = sender.send(Err(Error::Aborted));
        }
        for (_, queue) in self.start_session_calls_sending.drain() {
            for MethodSendingRecord { sender, .. } in queue {
                let _ = sender.send(Err(Error::Aborted));
            }
        }
        for (_, queue) in self.start_session_calls_receiving.drain() {
            for MethodReceivingRecord { sender, .. } in queue {
                let _ = sender.send(Err(Error::Aborted));
            }
        }
        *self = Self::new(self.timeout, self.capabilities.clone());
    }

    #[must_use]
    pub fn handle_tokens(&mut self, tokens: Vec<u8>) -> Vec<StackAction> {
        let mut actions = Vec::new();
        self.received_tokens.extend(tokens);
        loop {
            match extract_method::<MgmtMethodCall>(&mut self.received_tokens) {
                ExtractResult::Ok { value, tokens } => match value.params {
                    // The host should never receive a `StartSession`.
                    MgmtMethodCallParams::StartSession(_) => (),
                    MgmtMethodCallParams::SyncSession(sync_session) => {
                        self.handle_sync_session(&mut actions, sync_session, value.status, tokens)
                    }
                    MgmtMethodCallParams::CloseSession(close_session) => {
                        Self::handle_close_session(&mut actions, close_session, value.status);
                    }
                    MgmtMethodCallParams::Properties(properties_method) => {
                        self.handle_properties(&mut actions, properties_method, value.status)
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
        let packet = if let Some(MethodCallRecord { call, sender }) = self.method_calls.pop_front() {
            if call.len() > Properties::INITIAL.max_gross_packet_size.get() - PACKET_HEADER_LEN - SUB_PACKET_HEADER_LEN
            {
                let _ = sender.send(Err(Error::MethodTooLarge));
                None
            } else {
                match MgmtMethodCall::from_tokens(&call) {
                    Ok(call_detok) => {
                        let sequence_number = self.sequence_number.fetch_add();
                        match call_detok.params {
                            MgmtMethodCallParams::StartSession(start_session) => {
                                let hsn = start_session.host_session_id;
                                let record = MethodSendingRecord { sequence_number, sender };
                                self.start_session_calls_sending.entry(hsn).or_default().push_back(record);
                                Some(packetize_one(SessionId::MANAGEMENT, sequence_number, call))
                            }
                            MgmtMethodCallParams::SyncSession(_)
                            | MgmtMethodCallParams::CloseSession(_)
                            | MgmtMethodCallParams::Properties(_) => {
                                // You cannot send these method to the device:
                                // - SyncSession: only sent by the device
                                // - CloseSession: only sent by the device
                                // - Properties: could instruct the device to use capabilities that the protcol doesn't have.
                                let _ = sender.send(Err(Error::MethodNotAllowed));
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
        } else {
            None
        };

        // Remove timed out & get next deadline.
        let mut deadline = None;
        for (_, queue) in &mut self.start_session_calls_receiving {
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

    fn handle_sync_session(
        &mut self,
        actions: &mut Vec<StackAction>,
        sync_session: SyncSession,
        status: MethodStatus,
        tokens: Vec<u8>,
    ) {
        if let Some(queue) = self.start_session_calls_receiving.get_mut(&sync_session.host_session_id) {
            if let Some(record) = queue.pop_front() {
                if status == MethodStatus::Success {
                    let _ = record.sender.send(Ok(tokens));
                    let session_id = SessionId { hsn: sync_session.host_session_id, tsn: sync_session.sp_session_id };
                    // This is not entirely correct. The properties should be snapshot and saved when
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

    fn handle_properties(
        &mut self,
        actions: &mut Vec<StackAction>,
        properties_method: PropertiesMethod,
        status: MethodStatus,
    ) {
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
            actions.push(StackAction::Properties { connection: self.properties.clone(), device: properties });
        }
    }

    fn flush(&mut self, error: Error) {
        self.received_tokens.clear();
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

    pub fn capabilities(&self) -> &Properties {
        &self.capabilities
    }
}

#[derive(Debug, PartialEq, Eq)]
#[must_use]
pub enum StackAction {
    Spawn { session_id: SessionId, properties: Properties },
    NotifyAbort { session_id: SessionId },
    Properties { connection: Properties, device: Properties },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub enum Action {
    None,
    Sleep { until: Instant },
    Send(Vec<Packet>),
}

#[derive(Debug)]
pub struct MethodCallRecord {
    call: Vec<u8>,
    sender: Sender<Result<Vec<u8>, Error>>,
}

#[derive(Debug)]
struct MethodSendingRecord {
    /// The sequence number of the packet in which the method is being sent.
    sequence_number: SequenceNumber,
    sender: Sender<Result<Vec<u8>, Error>>,
}

#[derive(Debug)]
struct MethodReceivingRecord {
    /// The time when the message times out.
    deadline: Instant,
    sender: Sender<Result<Vec<u8>, Error>>,
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
    use sed_packet::token::ToTokens;
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

        mgmt.handle_method_call(call, sender);
        assert_that!(mgmt.poll_action(time), matches_pattern!(Action::None));
        assert_that!(receiver.try_recv(), ok(err(eq(&Error::MethodNotAllowed))));
    }

    #[test]
    fn start_session_completed_successfully() {
        let mut mgmt = Management::new(TIMEOUT, HOST_PROPERTIES);
        let time = Instant::now();
        let (sender, receiver) = channel();

        mgmt.handle_method_call(start_session_call(SESSION_ID), sender);
        assert_that!(
            mgmt.poll_action(time),
            matches_pattern!(Action::Send(eq(&vec![Packet {
                tper_session_number: 0,
                host_session_number: 0,
                sequence_number: 1,
                payload: vec![SubPacket {
                    kind: SubPacketKind::Data,
                    length: PhantomData,
                    payload: start_session_call(SESSION_ID)
                }],
                ..Default::default()
            }])))
        );

        mgmt.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        assert_that!(mgmt.poll_action(time), matches_pattern!(&Action::Sleep { until: eq(time + TIMEOUT) }));

        let stack_action = mgmt.handle_tokens(sync_session_call(SESSION_ID, MethodStatus::Success));
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

        mgmt.handle_method_call(start_session_call(SESSION_ID), sender);
        assert_that!(mgmt.poll_action(time), matches_pattern!(Action::Send(len(eq(1)))));

        mgmt.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        assert_that!(mgmt.poll_action(time), matches_pattern!(&Action::Sleep { until: eq(time + TIMEOUT) }));

        let stack_action = mgmt.handle_tokens(sync_session_call(SESSION_ID, MethodStatus::Fail));
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

        mgmt.handle_method_call(start_session_call(SESSION_ID), sender);
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

        mgmt.handle_method_call(start_session_call(SESSION_ID), sender);
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

        let stack_action = mgmt.handle_tokens(start_session_call(SESSION_ID));
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

        mgmt.handle_method_call(start_session_call(SESSION_ID), sender);
        assert_that!(
            mgmt.poll_action(time),
            matches_pattern!(Action::Send(eq(&vec![Packet {
                tper_session_number: 0,
                host_session_number: 0,
                sequence_number: 1,
                payload: vec![SubPacket {
                    kind: SubPacketKind::Data,
                    length: PhantomData,
                    payload: start_session_call(SESSION_ID)
                }],
                ..Default::default()
            }])))
        );

        mgmt.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        assert_that!(mgmt.poll_action(time), matches_pattern!(&Action::Sleep { until: eq(time + TIMEOUT) }));

        let stack_action = mgmt.handle_tokens(first_tokens);
        assert_that!(stack_action, eq(&vec![]));
        let stack_action = mgmt.handle_tokens(second_tokens);
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

        mgmt.handle_method_call(start_session_call(SESSION_ID), sender);
        assert_that!(
            mgmt.poll_action(time),
            matches_pattern!(Action::Send(eq(&vec![Packet {
                tper_session_number: 0,
                host_session_number: 0,
                sequence_number: 1,
                payload: vec![SubPacket {
                    kind: SubPacketKind::Data,
                    length: PhantomData,
                    payload: start_session_call(SESSION_ID)
                }],
                ..Default::default()
            }])))
        );

        mgmt.handle_iface_send_done(time, SequenceNumber(1), Ok(()));
        assert_that!(mgmt.poll_action(time), matches_pattern!(&Action::Sleep { until: eq(time + TIMEOUT) }));

        let stack_action = mgmt.handle_tokens(invalid_tokens);
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

        let stack_action = mgmt.handle_tokens(close_session_call(SESSION_ID));
        assert_that!(stack_action, eq(&vec![StackAction::NotifyAbort { session_id: SESSION_ID }]));
    }

    #[test]
    fn properties_received_without_host() {
        let mut mgmt = Management::new(TIMEOUT, HOST_PROPERTIES);

        let stack_action = mgmt.handle_tokens(properties_device_call(false));
        assert_that!(
            stack_action,
            eq(&vec![StackAction::Properties { connection: Properties::INITIAL, device: DEVICE_PROPERTIES }])
        );
    }

    #[test]
    fn properties_received_with_host() {
        let mut mgmt = Management::new(TIMEOUT, HOST_PROPERTIES);

        let stack_action = mgmt.handle_tokens(properties_device_call(true));
        assert_that!(
            stack_action,
            eq(&vec![StackAction::Properties { connection: CONNECTION_PROPERTIES, device: DEVICE_PROPERTIES }])
        );
    }
}
