use sed_packet::{Bytes, MaxBytes};
use sed_spec_macros::{DetokenizeStruct, TokenizeStruct};

use crate::{
    methods::properties::Properties,
    objects::{AuthorityRef, SpRef},
};

#[derive(Debug, DetokenizeStruct, TokenizeStruct)]
pub struct PropertiesHost {
    host_properties: Option<Properties>,
}

#[derive(Debug, DetokenizeStruct, TokenizeStruct)]
pub struct PropertiesTPer {
    properties: Properties,
    host_properties: Option<Properties>,
}

#[derive(Debug, DetokenizeStruct, TokenizeStruct)]
pub struct StartSession {
    host_session_id: u32,
    spid: SpRef,
    write: bool,
    host_challenge: Option<MaxBytes<32>>,
    host_exchange_authority: Option<AuthorityRef>,
    host_exchange_cert: Option<Bytes>,
    host_signing_authority: Option<AuthorityRef>,
    host_signing_cert: Option<Bytes>,
    session_timeout: Option<u32>,
    trans_timeout: Option<u32>,
    initial_credit: Option<u32>,
    signed_hash: Option<Bytes>,
}

#[derive(Debug, DetokenizeStruct, TokenizeStruct)]
pub struct SyncSession {
    host_session_id: u32,
    spsession_id: u32,
    sp_challenge: Option<Bytes>,
    sp_exchange_cert: Option<Bytes>,
    sp_signing_cert: Option<Bytes>,
    trans_timeout: Option<u32>,
    initial_credit: Option<u32>,
    signed_hash: Option<Bytes>,
}

#[derive(Debug, DetokenizeStruct, TokenizeStruct)]
pub struct CloseSession {
    remote_session_number: u32,
    local_session_number: u32,
}
