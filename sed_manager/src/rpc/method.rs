use crate::messaging::token::{Token, TokenStreamError};
use crate::messaging::uid::UID;
use crate::messaging::value::{Command, List, Value};
use crate::serialization::{Deserialize, Serialize};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MethodStatus {
    #[error("success")]
    Success = 0x00,
    #[error("permission denied")]
    NotAuthorized = 0x01,
    #[error("obsolete status code #0")]
    Obsolete0 = 0x02,
    #[error("security provider is busy")]
    SPBusy = 0x03,
    #[error("security provider has failed")]
    SPFailed = 0x04,
    #[error("security provider is disabled")]
    SPDisabled = 0x05,
    #[error("security provider is frozen")]
    SPFrozen = 0x06,
    #[error("no more sessions are available")]
    NoSessionsAvailable = 0x07,
    #[error("uniqueness conflict")]
    UniquenessConflict = 0x08,
    #[error("no more space is available")]
    InsufficientSpace = 0x09,
    #[error("no more rows are available")]
    InsufficientRows = 0x0A,
    #[error("invalid parameter")]
    InvalidParameter = 0x0C,
    #[error("obsolete status code #1")]
    Obsolete1 = 0x0D,
    #[error("obsolete status code #2")]
    Obsolete2 = 0x0E,
    #[error("the TPer experienced a malfunction")]
    TPerMalfunction = 0x0F,
    #[error("the transaction has failed")]
    TransactionFailure = 0x10,
    #[error("response overflow")]
    ResponseOverflow = 0x11,
    #[error("the authority is locked out")]
    AuthorityLockedOut = 0x12,
    #[error("unspecified failure")]
    Fail = 0x3F,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodCall {
    pub invoking_id: UID,
    pub method_id: UID,
    pub args: Vec<Value>,
    pub status: MethodStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodResult {
    pub results: Vec<Value>,
    pub status: MethodStatus,
}

impl From<MethodStatus> for Value {
    fn from(value: MethodStatus) -> Self {
        Value::from(value as u8)
    }
}

impl<'value> TryFrom<&'value Value> for MethodStatus {
    type Error = &'value Value;
    fn try_from(value: &'value Value) -> Result<Self, Self::Error> {
        let Ok(raw) = u8::try_from(value) else {
            return Err(value);
        };
        match raw {
            _ if raw == MethodStatus::Success as u8 => Ok(MethodStatus::Success),
            _ if raw == MethodStatus::NotAuthorized as u8 => Ok(MethodStatus::NotAuthorized),
            _ if raw == MethodStatus::Obsolete0 as u8 => Ok(MethodStatus::Obsolete0),
            _ if raw == MethodStatus::SPBusy as u8 => Ok(MethodStatus::SPBusy),
            _ if raw == MethodStatus::SPFailed as u8 => Ok(MethodStatus::SPFailed),
            _ if raw == MethodStatus::SPDisabled as u8 => Ok(MethodStatus::SPDisabled),
            _ if raw == MethodStatus::SPFrozen as u8 => Ok(MethodStatus::SPFrozen),
            _ if raw == MethodStatus::NoSessionsAvailable as u8 => Ok(MethodStatus::NoSessionsAvailable),
            _ if raw == MethodStatus::UniquenessConflict as u8 => Ok(MethodStatus::UniquenessConflict),
            _ if raw == MethodStatus::InsufficientSpace as u8 => Ok(MethodStatus::InsufficientSpace),
            _ if raw == MethodStatus::InsufficientRows as u8 => Ok(MethodStatus::InsufficientRows),
            _ if raw == MethodStatus::InvalidParameter as u8 => Ok(MethodStatus::InvalidParameter),
            _ if raw == MethodStatus::Obsolete1 as u8 => Ok(MethodStatus::Obsolete1),
            _ if raw == MethodStatus::Obsolete2 as u8 => Ok(MethodStatus::Obsolete2),
            _ if raw == MethodStatus::TPerMalfunction as u8 => Ok(MethodStatus::TPerMalfunction),
            _ if raw == MethodStatus::TransactionFailure as u8 => Ok(MethodStatus::TransactionFailure),
            _ if raw == MethodStatus::ResponseOverflow as u8 => Ok(MethodStatus::ResponseOverflow),
            _ if raw == MethodStatus::AuthorityLockedOut as u8 => Ok(MethodStatus::AuthorityLockedOut),
            _ if raw == MethodStatus::Fail as u8 => Ok(MethodStatus::Fail),
            _ => Err(value),
        }
    }
}

impl TryFrom<Value> for MethodStatus {
    type Error = Value;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match MethodStatus::try_from(&value) {
            Ok(method_status) => Ok(method_status),
            Err(_) => Err(value),
        }
    }
}

impl Serialize<Token> for MethodCall {
    type Error = TokenStreamError;
    fn serialize(&self, stream: &mut crate::serialization::OutputStream<Token>) -> Result<(), Self::Error> {
        Value::from(Command::Call).serialize(stream)?;
        Value::from(self.invoking_id).serialize(stream)?;
        Value::from(self.method_id).serialize(stream)?;
        Value::from(self.args.clone()).serialize(stream)?;
        Value::from(Command::EndOfData).serialize(stream)?;
        Value::from(vec![
            Value::from(self.status),
            Value::from(MethodStatus::Success),
            Value::from(MethodStatus::Success),
        ])
        .serialize(stream)?;
        Ok(())
    }
}

impl Serialize<Token> for MethodResult {
    type Error = TokenStreamError;
    fn serialize(&self, stream: &mut crate::serialization::OutputStream<Token>) -> Result<(), Self::Error> {
        Value::from(self.results.clone()).serialize(stream)?;
        Value::from(Command::EndOfData).serialize(stream)?;
        Value::from(vec![
            Value::from(self.status),
            Value::from(MethodStatus::Success),
            Value::from(MethodStatus::Success),
        ])
        .serialize(stream)?;
        Ok(())
    }
}

impl Deserialize<Token> for MethodCall {
    type Error = TokenStreamError;
    fn deserialize(stream: &mut crate::serialization::InputStream<Token>) -> Result<Self, Self::Error> {
        let call = Command::deserialize(stream)?;
        let invoking_id_value = Value::deserialize(stream)?;
        let method_id_value = Value::deserialize(stream)?;
        let args_value = Value::deserialize(stream)?;
        let eod = Command::deserialize(stream)?;
        let status_value = Value::deserialize(stream)?;

        if call != Command::Call {
            return Err(TokenStreamError::UnexpectedTag);
        };
        let Ok(invoking_id) = UID::try_from(invoking_id_value) else {
            return Err(TokenStreamError::ExpectedBytes);
        };
        let Ok(method_id) = UID::try_from(method_id_value) else {
            return Err(TokenStreamError::ExpectedBytes);
        };
        let Ok(args) = List::try_from(args_value) else {
            return Err(TokenStreamError::ExpectedList);
        };
        if eod != Command::EndOfData {
            return Err(TokenStreamError::UnexpectedTag);
        };
        let Ok(status_list) = List::try_from(status_value) else {
            return Err(TokenStreamError::ExpectedList);
        };
        let Some(status_value_0) = status_list.first() else {
            return Err(TokenStreamError::InvalidData);
        };
        let Ok(status) = MethodStatus::try_from(status_value_0) else {
            return Err(TokenStreamError::InvalidData);
        };

        Ok(MethodCall { invoking_id, method_id, args, status })
    }
}

impl Deserialize<Token> for MethodResult {
    type Error = TokenStreamError;
    fn deserialize(stream: &mut crate::serialization::InputStream<Token>) -> Result<Self, Self::Error> {
        let results_value = Value::deserialize(stream)?;
        let eod = Command::deserialize(stream)?;
        let status_value = Value::deserialize(stream)?;

        let Ok(results) = List::try_from(results_value) else {
            return Err(TokenStreamError::ExpectedList);
        };
        if eod != Command::EndOfData {
            return Err(TokenStreamError::UnexpectedTag);
        };
        let Ok(status_list) = List::try_from(status_value) else {
            // This check requires the status list to be a list,
            // but it does not check its contents. Not perfect but good enough.
            return Err(TokenStreamError::ExpectedList);
        };
        if status_list.len() != 3 {
            return Err(TokenStreamError::InvalidData);
        }
        let Ok(status) = MethodStatus::try_from(&status_list[0]) else {
            return Err(TokenStreamError::InvalidData);
        };
        Ok(MethodResult { results, status })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        messaging::token::Tag,
        serialization::{InputStream, OutputStream},
    };

    use super::*;

    fn example_method_call() -> MethodCall {
        MethodCall {
            invoking_id: 1_u64.into(),
            method_id: 2_u64.into(),
            args: vec![Value::from(6_u16), Value::from(7_u16)],
            status: MethodStatus::Fail,
        }
    }

    fn example_method_call_tokens() -> Vec<Token> {
        vec![
            Token { tag: Tag::Call, ..Default::default() },
            Token { tag: Tag::ShortAtom, is_byte: true, is_signed: false, data: vec![0, 0, 0, 0, 0, 0, 0, 1] },
            Token { tag: Tag::ShortAtom, is_byte: true, is_signed: false, data: vec![0, 0, 0, 0, 0, 0, 0, 2] },
            Token { tag: Tag::StartList, ..Default::default() },
            Token { tag: Tag::ShortAtom, is_byte: false, is_signed: false, data: vec![0, 6] },
            Token { tag: Tag::ShortAtom, is_byte: false, is_signed: false, data: vec![0, 7] },
            Token { tag: Tag::EndList, ..Default::default() },
            Token { tag: Tag::EndOfData, ..Default::default() },
            Token { tag: Tag::StartList, ..Default::default() },
            Token { tag: Tag::ShortAtom, is_byte: false, is_signed: false, data: vec![MethodStatus::Fail as u8] },
            Token { tag: Tag::ShortAtom, is_byte: false, is_signed: false, data: vec![0] },
            Token { tag: Tag::ShortAtom, is_byte: false, is_signed: false, data: vec![0] },
            Token { tag: Tag::EndList, ..Default::default() },
        ]
    }

    fn example_method_result() -> MethodResult {
        MethodResult { results: vec![Value::from(6_u16), Value::from(7_u16)], status: MethodStatus::NotAuthorized }
    }

    fn example_method_result_tokens() -> Vec<Token> {
        vec![
            Token { tag: Tag::StartList, ..Default::default() },
            Token { tag: Tag::ShortAtom, is_byte: false, is_signed: false, data: vec![0, 6] },
            Token { tag: Tag::ShortAtom, is_byte: false, is_signed: false, data: vec![0, 7] },
            Token { tag: Tag::EndList, ..Default::default() },
            Token { tag: Tag::EndOfData, ..Default::default() },
            Token { tag: Tag::StartList, ..Default::default() },
            Token {
                tag: Tag::ShortAtom,
                is_byte: false,
                is_signed: false,
                data: vec![MethodStatus::NotAuthorized as u8],
            },
            Token { tag: Tag::ShortAtom, is_byte: false, is_signed: false, data: vec![0] },
            Token { tag: Tag::ShortAtom, is_byte: false, is_signed: false, data: vec![0] },
            Token { tag: Tag::EndList, ..Default::default() },
        ]
    }

    #[test]
    fn serialize_method_call() {
        let mut stream = OutputStream::<Token>::new();
        example_method_call().serialize(&mut stream).unwrap();
        assert_eq!(stream.take(), example_method_call_tokens());
    }

    #[test]
    fn deserialize_method_call() {
        let mut stream = InputStream::<Token>::from(example_method_call_tokens());
        let method_call = MethodCall::deserialize(&mut stream).unwrap();
        let example = example_method_call();
        assert_eq!(method_call, example);
    }

    #[test]
    fn serialize_method_result() {
        let mut stream = OutputStream::<Token>::new();
        example_method_result().serialize(&mut stream).unwrap();
        assert_eq!(stream.take(), example_method_result_tokens());
    }

    #[test]
    fn deserialize_method_result() {
        let mut stream = InputStream::<Token>::from(example_method_result_tokens());
        let method_result = MethodResult::deserialize(&mut stream).unwrap();
        let example = example_method_result();
        assert_eq!(method_result, example);
    }
}
