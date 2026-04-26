use sed_packet::{
    Bytes, MaxBytes, Object, ObjectRef, Uid,
    token::{Detokenize, Detokenizer, MessageError as _, Tokenize, Tokenizer, ValueKind},
};
use sed_spec_macros::{DetokenizeStruct, TokenizeStruct};

use crate::methods::MethodParam;
use crate::methods::cell_block::CellBlock;
use crate::objects::{AceRef, AuthorityRef, MethodRef};
use crate::preconfig::core::shared::method_id;

//------------------------------------------------------------------------------
// Authenticate
//------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct Authenticate {
    authority: AuthorityRef,
    proof: Option<MaxBytes<32>>,
}

impl MethodParam for Authenticate {
    const METHOD_ID: Uid = method_id::AUTHENTICATE.to_uid();
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthenticateResult {
    Success(bool),
    Challenge(Bytes),
}

impl Tokenize for AuthenticateResult {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        match self {
            AuthenticateResult::Success(success) => success.tokenize(tokenizer),
            AuthenticateResult::Challenge(bytes) => bytes.tokenize(tokenizer),
        }
    }
}

impl Detokenize for AuthenticateResult {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        match detokenizer.peek_kind()? {
            ValueKind::Integer { .. } => bool::detokenize(detokenizer).map(|success| Self::Success(success)),
            ValueKind::Bytes => Bytes::detokenize(detokenizer).map(|bytes| Self::Challenge(bytes)),
            _ => Err(D::Error::message("expected either a boolean or bytes")),
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

impl<const TABLE: u64> MethodParam for Next<TABLE> {
    const METHOD_ID: Uid = method_id::NEXT.to_uid();
}

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct NextUntyped {
    pub where_: Option<Uid>,
    pub count: Option<u64>,
}

impl MethodParam for NextUntyped {
    const METHOD_ID: Uid = method_id::NEXT.to_uid();
}

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct NextResult<const TABLE: u64> {
    pub result: Vec<ObjectRef<TABLE>>,
}

//------------------------------------------------------------------------------
// GetACL
//------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct GetAcl {
    pub invoking_id: Uid,
    pub method_id: MethodRef,
}

impl MethodParam for GetAcl {
    const METHOD_ID: Uid = method_id::GET_ACL.to_uid();
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

impl MethodParam for GenKey {
    const METHOD_ID: Uid = method_id::GEN_KEY.to_uid();
}

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct GenKeyResult;

//------------------------------------------------------------------------------
// Revert
//------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct Revert;

impl MethodParam for Revert {
    const METHOD_ID: Uid = method_id::REVERT.to_uid();
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

impl MethodParam for RevertSp {
    const METHOD_ID: Uid = method_id::REVERT_SP.to_uid();
}

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct RevertSpResult;

//------------------------------------------------------------------------------
// Activate
//------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct Activate;

impl MethodParam for Activate {
    const METHOD_ID: Uid = method_id::ACTIVATE.to_uid();
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

impl MethodParam for Random {
    const METHOD_ID: Uid = method_id::RANDOM.to_uid();
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

impl MethodParam for Get {
    const METHOD_ID: Uid = method_id::GET.to_uid();
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

impl<O> MethodParam for SetObject<O>
where
    O: Object + Tokenize + Detokenize,
    O::Ref: Tokenize + Detokenize,
{
    const METHOD_ID: Uid = method_id::SET.to_uid();
}

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct SetBytes {
    pub where_: Option<u64>,
    pub values: Option<Bytes>,
}

impl MethodParam for SetBytes {
    const METHOD_ID: Uid = method_id::SET.to_uid();
}

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
pub struct SetResult;
