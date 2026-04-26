use std::sync::atomic::{AtomicUsize, Ordering};

use sed_packet::{
    Bytes, MaxBytes, Named, Uid,
    token::{Detokenize, Detokenizer, MessageError as _, Tokenize, Tokenizer, ValueKind},
};
use sed_spec_macros::{DetokenizeStruct, TokenizeStruct};

use crate::{
    methods::{MethodParam, properties::Properties},
    objects::{AuthorityRef, SecurityProviderRef},
    preconfig::core::shared::sm_method_id,
};

//------------------------------------------------------------------------------
// Properties
//------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertiesMethod {
    Host { host_properties: Option<Properties> },
    TPer { properties: Properties, host_properties: Option<Properties> },
}

impl MethodParam for PropertiesMethod {
    const METHOD_ID: Uid = sm_method_id::PROPERTIES;
}

impl Tokenize for PropertiesMethod {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        tokenizer.tokenize_list(|tokenizer| match self {
            PropertiesMethod::Host { host_properties } => {
                if let Some(host_properties) = host_properties {
                    Named { name: 0u8, value: host_properties }.tokenize(tokenizer)?;
                }
                Ok(())
            }
            PropertiesMethod::TPer { properties, host_properties } => {
                properties.tokenize(tokenizer)?;
                if let Some(host_properties) = host_properties {
                    Named { name: 0u8, value: host_properties }.tokenize(tokenizer)?;
                }
                Ok(())
            }
        })
    }
}

impl Detokenize for PropertiesMethod {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        let mut properties = None;
        let mut host_properties = None;
        let index = AtomicUsize::new(0); // I just want fetch_add, it doesn't need to be atomic.
        detokenizer.detokenize_list(|detokenizer| match index.fetch_add(1, Ordering::Relaxed) {
            0 => match detokenizer.peek_kind()? {
                ValueKind::Named => detokenize_host_properties(&mut host_properties, detokenizer),
                _ => {
                    properties = Some(Properties::detokenize(detokenizer)?);
                    Ok(())
                }
            },
            1 => detokenize_host_properties(&mut host_properties, detokenizer),
            _ => Err(D::Error::message("too many arguments received")),
        })?;
        match properties {
            Some(properties) => Ok(Self::TPer { properties, host_properties }),
            None => Ok(Self::Host { host_properties }),
        }
    }
}

fn detokenize_host_properties<D: Detokenizer>(
    host_properties: &mut Option<Properties>,
    detokenizer: &mut D,
) -> Result<(), <D as Detokenizer>::Error> {
    let Named { name, value } = Named::<u8, Properties>::detokenize(detokenizer)?;
    if name != 0 {
        return Err(D::Error::message("unexpected optional argument received"));
    }
    *host_properties = Some(value);
    Ok(())
}

//------------------------------------------------------------------------------
// StartSession
//------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct StartSession {
    pub host_session_id: u32,
    pub spid: SecurityProviderRef,
    pub write: bool,
    pub host_challenge: Option<MaxBytes<32>>,
    pub host_exchange_authority: Option<AuthorityRef>,
    pub host_exchange_cert: Option<Bytes>,
    pub host_signing_authority: Option<AuthorityRef>,
    pub host_signing_cert: Option<Bytes>,
    pub session_timeout: Option<u32>,
    pub trans_timeout: Option<u32>,
    pub initial_credit: Option<u32>,
    pub signed_hash: Option<Bytes>,
}

impl StartSession {
    pub fn new(host_session_id: u32, spid: SecurityProviderRef) -> Self {
        Self {
            host_session_id,
            spid,
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
        }
    }
}

impl MethodParam for StartSession {
    const METHOD_ID: Uid = sm_method_id::START_SESSION;
}

//------------------------------------------------------------------------------
// SyncSession
//------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct SyncSession {
    pub host_session_id: u32,
    pub sp_session_id: u32,
    pub sp_challenge: Option<Bytes>,
    pub sp_exchange_cert: Option<Bytes>,
    pub sp_signing_cert: Option<Bytes>,
    pub trans_timeout: Option<u32>,
    pub initial_credit: Option<u32>,
    pub signed_hash: Option<Bytes>,
}

impl SyncSession {
    pub fn new(host_session_id: u32, sp_session_id: u32) -> Self {
        Self {
            host_session_id,
            sp_session_id,
            sp_challenge: None,
            sp_exchange_cert: None,
            sp_signing_cert: None,
            trans_timeout: None,
            initial_credit: None,
            signed_hash: None,
        }
    }
}

impl MethodParam for SyncSession {
    const METHOD_ID: Uid = sm_method_id::SYNC_SESSION;
}

//------------------------------------------------------------------------------
// CloseSession
//------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct CloseSession {
    pub remote_session_number: u32,
    pub local_session_number: u32,
}

impl MethodParam for CloseSession {
    const METHOD_ID: Uid = sm_method_id::CLOSE_SESSION;
}
