//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::collections::BTreeMap;

use crate::rpc::{Error, PackagedMethod, Properties, SessionIdentifier};

use super::promise::Promise;
use super::shared::distribute::distribute;
use super::shared::pipe::{SinkPipe, SourcePipe};
use crate::rpc::protocol::shared::buffer::Buffer;
use crate::rpc::protocol::shared::gather::gather;
use assemble_com_packet::AssembleComPacket;
use assemble_packet::assemble_packet;
use assign_session_id::assign_session_id;
use serialize_method::serialize_method;

mod assemble_com_packet;
mod assemble_packet;
mod assign_session_id;
mod serialize_method;

type Request = Promise<PackagedMethod, PackagedMethod, Error>;
pub type Input = (SessionIdentifier, Request);
pub type Output = assemble_com_packet::Output;

pub struct SendPacket {
    sessions: BTreeMap<SessionIdentifier, Session>,
    gathered_packets: Buffer<assemble_com_packet::Input>,
    assemble_com_packet: AssembleComPacket,
}

struct Session {
    id: SessionIdentifier,
    properties: Properties,
    method: Buffer<serialize_method::Input>,
    serialized: Buffer<serialize_method::Output>,
    assembled: Buffer<assemble_packet::Output>,
    assigned: Buffer<assign_session_id::Output>,
    tracing_span: tracing::Span,
}

impl SendPacket {
    pub fn new(com_id: u16, com_id_ext: u16) -> Self {
        Self {
            sessions: BTreeMap::new(),
            gathered_packets: Buffer::new(),
            assemble_com_packet: AssembleComPacket::new(com_id, com_id_ext),
        }
    }

    pub fn open_session(&mut self, id: SessionIdentifier, properties: Properties) {
        if !self.sessions.contains_key(&id) {
            let session = Session::new(id, properties);
            self.sessions.insert(id, session);
        }
    }

    pub fn close_session(&mut self, id: SessionIdentifier) {
        if let Some(session) = self.sessions.get_mut(&id) {
            session.close();
        }
    }

    pub fn abort_session(&mut self, id: SessionIdentifier) {
        if let Some(session) = self.sessions.get_mut(&id) {
            session.abort();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    pub fn update(
        &mut self,
        input: &mut dyn SourcePipe<Input>,
        output: &mut dyn SinkPipe<Output>,
        done: &mut dyn SinkPipe<SessionIdentifier>,
    ) {
        distribute(input, &mut Self::distribution_table(&mut self.sessions), |(id, _)| *id, |(_, pr)| pr);
        for (_, session) in &mut self.sessions {
            session.update();
        }
        gather(&mut Self::gather_list(&mut self.sessions), &mut self.gathered_packets);
        self.assemble_com_packet.update(&mut self.gathered_packets, output);
        self.cleanup_sessions(output, done);
    }

    fn distribution_table<'a>(
        sessions: &'a mut BTreeMap<SessionIdentifier, Session>,
    ) -> BTreeMap<SessionIdentifier, &'a mut dyn SinkPipe<Request>> {
        sessions
            .iter_mut()
            .map(|(id, session)| (*id, session.input() as &mut dyn SinkPipe<Request>))
            .collect()
    }

    fn gather_list<'a>(
        sessions: &'a mut BTreeMap<SessionIdentifier, Session>,
    ) -> Vec<&'a mut dyn SourcePipe<assign_session_id::Output>> {
        sessions
            .values_mut()
            .map(|session| &mut session.assigned as &mut dyn SourcePipe<assign_session_id::Output>)
            .collect()
    }

    fn cleanup_sessions(
        &mut self,
        com_packet: &mut dyn SinkPipe<Output>,
        done_out: &mut dyn SinkPipe<SessionIdentifier>,
    ) {
        let done: Vec<_> = self.sessions.iter().filter(|(_, session)| session.is_done()).map(|(id, _)| *id).collect();
        for id in &done {
            self.sessions.remove(id);
            done_out.push(*id);
        }
        if !done.is_empty() && self.sessions.is_empty() {
            com_packet.close();
            done_out.close();
        }
    }
}

impl Session {
    pub fn new(id: SessionIdentifier, properties: Properties) -> Self {
        let tracing_span = tracing::span!(tracing::Level::DEBUG, "session", hsn = id.hsn, tsn = id.tsn);
        {
            let _guard = tracing_span.enter();
            tracing::event!(tracing::Level::DEBUG, "[send] Started");
        };
        Self {
            id,
            properties,
            method: Buffer::new(),
            serialized: Buffer::new(),
            assembled: Buffer::new(),
            assigned: Buffer::new(),
            tracing_span,
        }
    }

    pub fn update(&mut self) {
        let _guard = self.tracing_span.enter();
        serialize_method(&mut self.method, &mut self.serialized, &self.properties);
        assemble_packet(&mut self.serialized, &mut self.assembled, &self.properties);
        assign_session_id(&mut self.assembled, &mut self.assigned, &self.id);
    }

    pub fn input(&mut self) -> &mut Buffer<serialize_method::Input> {
        &mut self.method
    }

    pub fn close(&mut self) {
        self.method.close();
    }

    pub fn abort(&mut self) {
        self.close();
        self.assigned.clear();
        self.assigned.close();
    }

    pub fn is_done(&self) -> bool {
        self.assigned.is_done()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _guard = self.tracing_span.enter();
        tracing::event!(tracing::Level::DEBUG, done = self.is_done(), "[send] Ended");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use core::task::Poll;

    use crate::rpc::protocol::{promise::Promise, shared::buffer::Buffer};
    use crate::rpc::{PackagedMethod, Properties, SessionIdentifier};

    fn setup() -> (Buffer<Input>, SendPacket, Buffer<Output>, Buffer<SessionIdentifier>) {
        (Buffer::new(), SendPacket::new(2048, 0), Buffer::new(), Buffer::new())
    }

    #[test]
    fn initial_state() {
        let (mut input, mut node, mut output, mut done) = setup();
        node.update(&mut input, &mut output, &mut done);
        assert!(!output.is_done());
        assert!(!done.is_done());
    }

    #[test]
    fn invalid_session() {
        let id = SessionIdentifier { hsn: 0, tsn: 0 };
        let (mut input, mut node, mut output, mut done) = setup();
        input.push((id, Promise::new(PackagedMethod::EndOfSession, vec![])));
        node.update(&mut input, &mut output, &mut done);
        assert!(!output.is_done());
        assert!(!done.is_done());
    }

    #[test]
    fn active_session() {
        let id = SessionIdentifier { hsn: 0, tsn: 0 };
        let (mut input, mut node, mut output, mut done) = setup();
        input.push((id, Promise::new(PackagedMethod::EndOfSession, vec![])));
        node.open_session(id, Properties::ASSUMED);
        node.update(&mut input, &mut output, &mut done);
        assert_eq!(node.sessions.len(), 1);
        assert!(output.pop().is_ready());
        assert!(output.pop().is_pending());
        assert!(done.pop().is_pending());
        assert!(!output.is_done());
        assert!(!done.is_done());
    }

    #[test]
    fn closing_session() {
        let id = SessionIdentifier { hsn: 0, tsn: 0 };
        let (mut input, mut node, mut output, mut done) = setup();
        node.open_session(id, Properties::ASSUMED);
        node.update(&mut input, &mut output, &mut done);
        node.close_session(id);
        node.update(&mut input, &mut output, &mut done);
        assert!(node.sessions.is_empty());
        assert!(output.is_done());
        assert_eq!(done.pop(), Poll::Ready(Some(id)));
        assert!(done.is_done());
    }
}
