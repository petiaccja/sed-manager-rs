use sed_packet::{
    Bytes, MaxBytes, Object, ObjectRef, Uid,
    token::{Detokenize, Detokenizer, MessageError as _, Tokenize, Tokenizer, ValueKind},
};
use sed_spec_macros::{DetokenizeStruct, TokenizeStruct};

use crate::{
    methods::cell_block::CellBlock,
    objects::{AceRef, AuthorityRef, MethodRef},
};

#[derive(Debug, DetokenizeStruct, TokenizeStruct)]
pub struct Authenticate {
    authority: AuthorityRef,
    proof: Option<MaxBytes<32>>,
}

#[derive(Debug)]
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

#[derive(Debug, DetokenizeStruct, TokenizeStruct)]
pub struct Next<const TABLE: u64> {
    where_: Option<ObjectRef<TABLE>>,
    count: Option<u64>,
}

#[derive(Debug, DetokenizeStruct, TokenizeStruct)]
pub struct NextResult<const TABLE: u64> {
    result: Vec<ObjectRef<TABLE>>,
}

#[derive(Debug, DetokenizeStruct, TokenizeStruct)]
pub struct GetAcl {
    invoking_id: Uid,
    method_id: MethodRef,
}

// This is tokenized in a weird way. The fact that the "access control list" is
// a list itself is ignored, and the ACE references are tokenized directly into
// the method argument list.
#[derive(Debug)]
pub struct GetAclResult {
    acl: Vec<AceRef>,
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

#[derive(Debug, DetokenizeStruct, TokenizeStruct)]
pub struct GenKey {
    public_exponent: Option<u64>,
    pin_length: Option<u8>,
}

#[derive(Debug, DetokenizeStruct, TokenizeStruct)]
pub struct GenKeyResult;

#[derive(Debug, DetokenizeStruct, TokenizeStruct)]
pub struct Revert;

#[derive(Debug, DetokenizeStruct, TokenizeStruct)]
pub struct RevertResult;

#[derive(Debug, DetokenizeStruct, TokenizeStruct)]
pub struct RevertSp {
    keep_global_range_key: Option<bool>,
}

#[derive(Debug, DetokenizeStruct, TokenizeStruct)]
pub struct RevertSpResult;

#[derive(Debug, DetokenizeStruct, TokenizeStruct)]
pub struct Activate;

#[derive(Debug, DetokenizeStruct, TokenizeStruct)]
pub struct ActivateResult;

#[derive(Debug, DetokenizeStruct, TokenizeStruct)]
pub struct Random {
    count: u64,
    buffer_out: Option<CellBlock>,
}

#[derive(Debug, DetokenizeStruct, TokenizeStruct)]
pub struct RandomResult {
    result: Bytes,
}

#[derive(Debug, DetokenizeStruct, TokenizeStruct)]
pub struct Get {
    cell_block: CellBlock,
}

#[derive(Debug, DetokenizeStruct, TokenizeStruct)]
pub struct GetBytesResult {
    result: Bytes,
}

#[derive(Debug, DetokenizeStruct, TokenizeStruct)]
pub struct GetObjectResult<O: Tokenize + Detokenize> {
    result: O,
}

#[derive(Debug, DetokenizeStruct, TokenizeStruct)]
pub struct SetObject<O>
where
    O: Object + Tokenize + Detokenize,
    O::Ref: Tokenize + Detokenize,
{
    where_: Option<O::Ref>,
    values: Option<O>,
}

#[derive(Debug, DetokenizeStruct, TokenizeStruct)]
pub struct SetBytes {
    where_: Option<u64>,
    values: Option<Bytes>,
}

#[derive(Debug, DetokenizeStruct, TokenizeStruct)]
pub struct SetResult;
