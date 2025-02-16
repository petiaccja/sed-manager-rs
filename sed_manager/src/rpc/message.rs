use crate::messaging::token::{Tag, Token, TokenStreamError};
use crate::messaging::value::Command;
use crate::serialization::{Deserialize, InputStream, ItemRead, OutputStream, Serialize};

use super::method::{MethodCall, MethodResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackagedMethod {
    Call(MethodCall),
    Result(MethodResult),
    EndOfSession,
}

impl Serialize<Token> for PackagedMethod {
    type Error = TokenStreamError;
    fn serialize(&self, stream: &mut OutputStream<Token>) -> Result<(), Self::Error> {
        match self {
            PackagedMethod::Call(method_call) => method_call.serialize(stream),
            PackagedMethod::Result(method_result) => method_result.serialize(stream),
            PackagedMethod::EndOfSession => Command::EndOfSession.serialize(stream),
        }
    }
}

impl Deserialize<Token> for PackagedMethod {
    type Error = TokenStreamError;
    fn deserialize(stream: &mut InputStream<Token>) -> Result<Self, Self::Error> {
        let Ok(first) = stream.peek_one() else {
            return Err(TokenStreamError::EndOfStream);
        };
        match first.tag {
            Tag::Call => Ok(PackagedMethod::Call(MethodCall::deserialize(stream)?)),
            Tag::StartList => Ok(PackagedMethod::Result(MethodResult::deserialize(stream)?)),
            Tag::EndOfSession => {
                let _ = stream.read_one();
                Ok(PackagedMethod::EndOfSession)
            }
            _ => Err(TokenStreamError::UnexpectedTag),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::rpc::MethodStatus;
    use crate::serialization::Error as SerializeError;
    use crate::serialization::SeekAlways;

    use super::*;

    #[test]
    fn serialize_packaged_method_call() -> Result<(), SerializeError> {
        let call = PackagedMethod::Call(MethodCall {
            invoking_id: 0xFFu64.into(),
            method_id: 0xEFu64.into(),
            args: vec![],
            status: MethodStatus::Fail,
        });
        let mut os = OutputStream::<Token>::new();
        call.serialize(&mut os)?;
        let stream_len = os.len();
        let mut is = InputStream::from(os.take());
        let copy = PackagedMethod::deserialize(&mut is)?;
        assert_eq!(call, copy);
        assert_eq!(is.pos(), stream_len);
        Ok(())
    }

    #[test]
    fn serialize_packaged_method_result() -> Result<(), SerializeError> {
        let call = PackagedMethod::Result(MethodResult { results: vec![], status: MethodStatus::Fail });
        let mut os = OutputStream::<Token>::new();
        call.serialize(&mut os)?;
        let stream_len = os.len();
        let mut is = InputStream::from(os.take());
        let copy = PackagedMethod::deserialize(&mut is)?;
        assert_eq!(call, copy);
        assert_eq!(is.pos(), stream_len);
        Ok(())
    }

    #[test]
    fn serialize_packaged_method_eos() -> Result<(), SerializeError> {
        let call = PackagedMethod::EndOfSession;
        let mut os = OutputStream::<Token>::new();
        call.serialize(&mut os)?;
        let stream_len = os.len();
        let mut is = InputStream::from(os.take());
        let copy = PackagedMethod::deserialize(&mut is)?;
        assert_eq!(call, copy);
        assert_eq!(is.pos(), stream_len);
        Ok(())
    }
}
