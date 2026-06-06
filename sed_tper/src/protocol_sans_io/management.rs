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
    protocol_sans_io::{
        sequence_number::SequenceNumber,
        utility::{min_deadline, packetize_one},
    },
};

pub struct Management {
    sequence_number: SequenceNumber,
    timeout: Duration,
    capabilities: Properties,
    properties: Properties,
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
            start_session_calls_sending: HashMap::new(),
            start_session_calls_receiving: HashMap::new(),
            received_tokens: VecDeque::new(),
        }
    }

    pub fn handle_method_call(&mut self, call: Vec<u8>, sender: Sender<Result<Vec<u8>, Error>>) -> Option<Packet> {
        if call.len() > Properties::INITIAL.max_gross_packet_size.get() - PACKET_HEADER_LEN - SUB_PACKET_HEADER_LEN {
            let _ = sender.send(Err(Error::MethodTooLarge));
            return None;
        }
        match MgmtMethodCall::from_tokens(&call) {
            Ok(call_detok) => {
                let sequence_number = self.sequence_number.fetch_add();
                match call_detok.params {
                    MgmtMethodCallParams::StartSession(start_session) => {
                        let hsn = start_session.host_session_id;
                        let record = MethodSendingRecord { sequence_number, sender };
                        self.start_session_calls_sending.entry(hsn).or_default().push_back(record);
                    }
                    MgmtMethodCallParams::SyncSession(_) => (),
                    MgmtMethodCallParams::CloseSession(_) => (),
                    MgmtMethodCallParams::Properties(_) => (),
                };
                Some(packetize_one(SessionId::MANAGEMENT, sequence_number, call))
            }
            Err(err) => {
                let _ = sender.send(Err(err.into()));
                None
            }
        }
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

    #[must_use]
    pub fn handle_tokens(&mut self, tokens: Vec<u8>) -> Vec<ManagementAction> {
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

    fn handle_sync_session(
        &mut self,
        actions: &mut Vec<ManagementAction>,
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
                    actions.push(ManagementAction::Spawn { session_id, properties: self.properties.clone() });
                } else {
                    let _ = record.sender.send(Err(status.into()));
                }
            }
            if queue.is_empty() {
                self.start_session_calls_receiving.remove(&sync_session.host_session_id);
            }
        }
    }

    fn handle_close_session(actions: &mut Vec<ManagementAction>, close_session: CloseSession, status: MethodStatus) {
        if status == MethodStatus::Success {
            let session_id =
                SessionId { hsn: close_session.remote_session_number, tsn: close_session.local_session_number };
            actions.push(ManagementAction::NotifyAbort { session_id });
        }
    }

    fn handle_properties(
        &mut self,
        actions: &mut Vec<ManagementAction>,
        properties_method: PropertiesMethod,
        status: MethodStatus,
    ) {
        if status == MethodStatus::Success
            && let PropertiesMethod::TPer { properties, .. } = properties_method
        {
            self.properties = Properties::common(&self.capabilities, &properties);
            actions.push(ManagementAction::Properties { connection: self.properties.clone(), device: properties });
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

    pub fn poll_timeout(&self) -> Option<Instant> {
        let mut deadline = None;
        for (_, queue) in &self.start_session_calls_receiving {
            if let Some(record) = queue.front() {
                deadline = min_deadline(deadline, Some(record.deadline))
            }
        }
        deadline
    }

    pub fn notify_time(&mut self, time: Instant) {
        for (_, queue) in &mut self.start_session_calls_receiving {
            while let Some(record) = queue.pop_front_if(|record| record.deadline < time) {
                let _ = record.sender.send(Err(Error::TimedOut));
            }
        }
        self.start_session_calls_receiving.retain(|_, queue| !queue.is_empty());
    }
}

#[must_use]
pub enum ManagementAction {
    Spawn {
        session_id: SessionId,
        properties: Properties,
    },
    NotifyAbort {
        session_id: SessionId,
    },
    #[expect(unused)]
    Properties {
        connection: Properties,
        device: Properties,
    },
}

struct MethodSendingRecord {
    /// The sequence number of the packet in which the method is being sent.
    sequence_number: SequenceNumber,
    sender: Sender<Result<Vec<u8>, Error>>,
}

struct MethodReceivingRecord {
    /// The time when the message times out.
    deadline: Instant,
    sender: Sender<Result<Vec<u8>, Error>>,
}
