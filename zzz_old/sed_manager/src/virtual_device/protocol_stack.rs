//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::sync::atomic::{AtomicU32, Ordering};
use std::collections::HashMap;

use crate::rpc::{Properties, SessionIdentifier};
use crate::spec::column_types::{AuthorityRef, SPRef};

const INITIAL_TSN: u32 = 500;
const INITIAL_PROPERTIES: Properties = Properties::ASSUMED;

pub struct ProtocolStack {
    sp_sessions: HashMap<SessionIdentifier, SPSessionData>,
    next_tsn: AtomicU32,
    pub capabilities: Properties,
    pub properties: Properties,
}

pub struct SPSessionData {
    pub sp: SPRef,
    pub authenticated: Vec<AuthorityRef>,
}

impl ProtocolStack {
    pub fn new(capabilities: Properties) -> Self {
        Self {
            sp_sessions: HashMap::new(),
            next_tsn: INITIAL_TSN.into(),
            capabilities,
            properties: INITIAL_PROPERTIES,
        }
    }

    pub fn add_session(&mut self, sp: SPRef, hsn: u32) -> SessionIdentifier {
        let session_id = self.next_session_id(hsn);
        let session = SPSessionData { sp, authenticated: vec![] };
        self.sp_sessions.insert(session_id, session);
        session_id
    }

    pub fn remove_session(&mut self, session_id: SessionIdentifier) {
        self.sp_sessions.remove(&session_id);
    }

    pub fn list_sessions(&self) -> impl Iterator<Item = &SessionIdentifier> {
        self.sp_sessions.keys()
    }

    pub fn get_session(&self, session_id: SessionIdentifier) -> Option<&SPSessionData> {
        self.sp_sessions.get(&session_id)
    }

    pub fn get_session_mut(&mut self, session_id: SessionIdentifier) -> Option<&mut SPSessionData> {
        self.sp_sessions.get_mut(&session_id)
    }

    pub fn prune_sessions(&mut self, sp: SPRef) -> Vec<SessionIdentifier> {
        let pruned_session_ids: Vec<_> = self
            .sp_sessions
            .iter()
            .filter(|(_, session)| session.sp == sp)
            .map(|(session_id, _)| *session_id)
            .collect();
        pruned_session_ids.iter().for_each(|session_id| {
            self.sp_sessions.remove(session_id);
        });
        pruned_session_ids
    }

    pub fn reset(&mut self) {
        self.sp_sessions.clear();
        self.next_tsn.store(INITIAL_TSN, Ordering::Relaxed);
        self.properties = INITIAL_PROPERTIES;
    }

    fn next_session_id(&self, hsn: u32) -> SessionIdentifier {
        let tsn = self.next_tsn.fetch_add(1, Ordering::Relaxed);
        SessionIdentifier { hsn, tsn }
    }
}
