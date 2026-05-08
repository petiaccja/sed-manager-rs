use std::convert::Infallible;

use sed_packet::{
    Bytes, MaxBytes, Object, ObjectRef, Uid,
    token::{Detokenize, Detokenizer, MessageError as _, Tokenize, Tokenizer, ValueKind},
};
use sed_spec_macros::{DetokenizeStruct, TokenizeStruct};

use crate::methods::SessionMethodParam;
use crate::methods::cell_block::CellBlock;
use crate::objects::{AceRef, AuthorityRef, MethodRef};
use crate::preconfig::core::shared::method_id;

//------------------------------------------------------------------------------
// Authenticate
//------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct Authenticate {
    pub authority: AuthorityRef,
    pub proof: Option<MaxBytes<32>>,
}

impl SessionMethodParam for Authenticate {
    const METHOD_ID: MethodRef = method_id::AUTHENTICATE;
    type Result = AuthenticateResult;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthenticateResult {
    Success(bool),
    Challenge(Bytes),
}

impl Tokenize for AuthenticateResult {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        tokenizer.tokenize_list(|tokenizer| match self {
            AuthenticateResult::Success(success) => success.tokenize(tokenizer),
            AuthenticateResult::Challenge(bytes) => bytes.tokenize(tokenizer),
        })
    }
}

impl Detokenize for AuthenticateResult {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        let mut result = None;
        detokenizer.detokenize_list(|detokenizer| {
            if result.is_some() {
                return Err(D::Error::message("too many parameters in method result"));
            }
            match detokenizer.peek_kind()? {
                ValueKind::Integer { .. } => bool::detokenize(detokenizer).map(|success| Self::Success(success)),
                ValueKind::Bytes => Bytes::detokenize(detokenizer).map(|bytes| Self::Challenge(bytes)),
                _ => Err(D::Error::message("expected either a boolean or bytes")),
            }
            .map(|result_| {
                result = Some(result_);
                ()
            })
        })?;
        match result {
            Some(result) => Ok(result),
            None => Err(D::Error::message("empty method result")),
        }
    }
}

//------------------------------------------------------------------------------
// Next
//------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct Next<const TABLE: u64> {
    pub where_: Option<ObjectRef<TABLE>>,
    pub count: Option<u64>,
}

impl<const TABLE: u64> SessionMethodParam for Next<TABLE> {
    const METHOD_ID: MethodRef = method_id::NEXT;
    type Result = NextResult<TABLE>;
}

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct NextResult<const TABLE: u64> {
    pub result: Vec<ObjectRef<TABLE>>,
}

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct NextUntyped {
    pub where_: Option<Uid>,
    pub count: Option<u64>,
}

impl SessionMethodParam for NextUntyped {
    const METHOD_ID: MethodRef = method_id::NEXT;
    type Result = NextResultUntyped;
}

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct NextResultUntyped {
    pub result: Vec<Uid>,
}

//------------------------------------------------------------------------------
// GetACL
//------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct GetAcl {
    pub invoking_id: Uid,
    pub method_id: MethodRef,
}

impl SessionMethodParam for GetAcl {
    const METHOD_ID: MethodRef = method_id::GET_ACL;
    type Result = GetAclResult;
}

// This is tokenized in a weird way. The fact that the "access control list" is
// a list itself is ignored, and the ACE references are tokenized directly into
// the method argument list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetAclResult {
    pub acl: Vec<AceRef>,
}

impl Tokenize for GetAclResult {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        self.acl.tokenize(tokenizer)
    }
}

impl Detokenize for GetAclResult {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        Ok(Self { acl: <_>::detokenize(detokenizer)? })
    }
}

//------------------------------------------------------------------------------
// GenKey
//------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct GenKey {
    pub public_exponent: Option<u64>,
    pub pin_length: Option<u8>,
}

impl SessionMethodParam for GenKey {
    const METHOD_ID: MethodRef = method_id::GEN_KEY;
    type Result = GenKeyResult;
}

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct GenKeyResult;

//------------------------------------------------------------------------------
// Revert
//------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct Revert;

impl SessionMethodParam for Revert {
    const METHOD_ID: MethodRef = method_id::REVERT;
    type Result = RevertResult;
}

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct RevertResult;

//------------------------------------------------------------------------------
// RevertSP
//------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct RevertSp {
    pub keep_global_range_key: Option<bool>,
}

impl SessionMethodParam for RevertSp {
    const METHOD_ID: MethodRef = method_id::REVERT_SP;
    type Result = RevertSpResult;
}

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct RevertSpResult;

//------------------------------------------------------------------------------
// Activate
//------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct Activate;

impl SessionMethodParam for Activate {
    const METHOD_ID: MethodRef = method_id::ACTIVATE;
    type Result = ActivateResult;
}

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct ActivateResult;

//------------------------------------------------------------------------------
// Random
//------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct Random {
    pub count: u64,
    pub buffer_out: Option<CellBlock>,
}

impl SessionMethodParam for Random {
    const METHOD_ID: MethodRef = method_id::RANDOM;
    type Result = RandomResult;
}

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct RandomResult {
    pub result: Bytes,
}

//------------------------------------------------------------------------------
// Get
//------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct Get {
    pub cell_block: CellBlock,
}

impl SessionMethodParam for Get {
    const METHOD_ID: MethodRef = method_id::GET;
    type Result = Infallible;
}

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct GetBytesResult {
    pub result: Bytes,
}

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct GetObjectResult<O: Tokenize + Detokenize> {
    pub result: O,
}

//------------------------------------------------------------------------------
// Set
//------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct SetObject<O>
where
    O: Object + Tokenize + Detokenize,
    O::Ref: Tokenize + Detokenize,
{
    pub where_: Option<O::Ref>,
    pub values: Option<O>,
}

impl<O> SessionMethodParam for SetObject<O>
where
    O: Object + Tokenize + Detokenize,
    O::Ref: Tokenize + Detokenize,
{
    const METHOD_ID: MethodRef = method_id::SET;
    type Result = SetResult;
}

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct SetBytes {
    pub where_: Option<u64>,
    pub values: Option<Bytes>,
}

impl SessionMethodParam for SetBytes {
    const METHOD_ID: MethodRef = method_id::SET;
    type Result = SetResult;
}

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct SetResult;
