use std::convert::Infallible;

use sed_packet::token::{Command, Detokenize, Detokenizer, MessageError as _, Tokenize, Tokenizer};

use crate::methods::MethodStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodResult<Parameters>(pub Result<Parameters, MethodStatus>);

impl<Parameters> Tokenize for MethodResult<Parameters>
where
    Parameters: Tokenize,
{
    /// Serialize the method result into tokenized bytes.
    ///
    /// For successful results, the parameters are serialized as usual. For
    /// results indicating failure, an empty list is serialized with the error
    /// status.
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        let status = match &self.0 {
            Ok(parameters) => {
                parameters.tokenize(tokenizer)?;
                MethodStatus::Success
            }
            Err(status) => {
                assert_ne!(status, &MethodStatus::Success, "you should never specify success for the error variant");
                Vec::<Infallible>::new().tokenize(tokenizer)?;
                *status
            }
        };
        Command::EndOfData.tokenize(tokenizer)?;
        vec![status, MethodStatus::Success, MethodStatus::Success].tokenize(tokenizer)?;
        Ok(())
    }
}

impl<Parameters> Detokenize for MethodResult<Parameters>
where
    Parameters: Detokenize,
{
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        // The parameter list may be empty for a failed invocation, meaning the
        // detokenization could fail. The way detokenization is (and must be)
        // implemented for method parameters it consumes all tokens, including
        // the END_LIST token. This way, detokenization of subsequent items
        // can be continued.
        let maybe_parameters = Parameters::detokenize(detokenizer);
        let eod_command = Command::detokenize(detokenizer)?;
        if eod_command != Command::EndOfData {
            return Err(D::Error::message("expected an END_OF_DATA token"));
        }
        let status = Vec::<MethodStatus>::detokenize(detokenizer)?;
        let Some(status) = status.first().cloned() else {
            return Err(D::Error::message("received empty method status list"));
        };
        match (maybe_parameters, status) {
            (Ok(parameters), MethodStatus::Success) => Ok(Self(Ok(parameters))),
            (Ok(_), status) => Ok(Self(Err(status))),
            (Err(err), MethodStatus::Success) => Err(err),
            (Err(_), status) => Ok(Self(Err(status))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;
    use sed_packet::token::{Error as TokenError, FromTokens as _, ToTokens as _};
    use sed_spec_macros::{DetokenizeStruct, TokenizeStruct};

    #[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
    struct Parameters {
        a: u8,
        b: Option<u8>,
    }

    #[rstest]
    #[case(MethodResult(Ok(Parameters{a: 1, b: None})), &[0xF0, 0x01, 0xF1, 0xF9, 0xF0, 0x00, 0x00, 0x00, 0xF1])]
    #[case(MethodResult(Ok(Parameters{a: 1, b: Some(2)})), &[0xF0, 0x01, 0xF2, 0x00, 0x02, 0xF3, 0xF1, 0xF9, 0xF0, 0x00, 0x00, 0x00, 0xF1])]
    #[case(MethodResult(Err(MethodStatus::Fail)), &[0xF0, 0xF1, 0xF9, 0xF0, 0x3F, 0x00, 0x00, 0xF1])]
    fn tokenize(#[case] value: MethodResult<Parameters>, #[case] bytes: &[u8]) {
        assert_eq!(value.to_tokens().unwrap(), bytes);
        assert_eq!(<MethodResult<Parameters>>::from_tokens(bytes).unwrap(), value);
    }

    #[rstest]
    #[case(Ok(MethodResult(Err(MethodStatus::Fail))), &[0xF0, 0x01, 0xF1, 0xF9, 0xF0, 0x3F, 0x00, 0x00, 0xF1])]
    #[case(Err(TokenError::Custom("mandatory field a missing".into())), &[0xF0, 0xF1, 0xF9, 0xF0, 0x00, 0x00, 0x00, 0xF1])]
    fn detokenize_edge_cases(#[case] value: Result<MethodResult<Parameters>, TokenError>, #[case] bytes: &[u8]) {
        assert_eq!(<MethodResult<Parameters>>::from_tokens(bytes), value);
    }
}
