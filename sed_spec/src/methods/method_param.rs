use sed_packet::{Uid, token::Detokenize, token::Error as TokenError, token::FromTokens as _};

use crate::{
    methods::{MethodCall, MethodResult, MethodStatus},
    objects::MethodRef,
};

pub trait MethodParam {
    const METHOD_ID: Uid;

    fn method_id(&self) -> Uid {
        Self::METHOD_ID
    }
}

pub trait SessionMethodParam {
    const METHOD_ID: MethodRef;
    type Result;

    fn method_id(&self) -> MethodRef {
        Self::METHOD_ID
    }

    fn to_call(self, invoking_id: Uid) -> MethodCall<Self>
    where
        Self: Sized,
    {
        MethodCall { invoking_id, method_id: Self::METHOD_ID.to_uid(), parameters: self, status: MethodStatus::Success }
    }

    fn result_from_tokens(tokens: &[u8]) -> Result<Result<Self::Result, MethodStatus>, TokenError>
    where
        Self::Result: Detokenize,
    {
        MethodResult::<Self::Result>::from_tokens(tokens).map(|result| result.0)
    }
}
