use std::convert::Infallible;

use sed_packet::token::{Command, Detokenize, Detokenizer, MessageError as _, Tokenize, Tokenizer};

use crate::methods::MethodStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodResult<ParamList>(pub Result<ParamList, MethodStatus>);

impl<ParamList> Tokenize for MethodResult<ParamList>
where
    ParamList: Tokenize,
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

impl<ParamList> Detokenize for MethodResult<ParamList>
where
    ParamList: Detokenize,
{
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        // The parameter list may be empty (i.e. [SL, EL]) for failed method
        // invocations (i.e. when the status is not SUCCESS).
        // In this case, deserializing the parameters will likely fail, as an
        // empty list is not valid as the parameter structure. Regardless, the
        // deserialization must continue to see if the status code is
        // non-success, in which case the method result is still valid.
        match ParamList::detokenize(detokenizer) {
            Ok(param_list) => {
                let eod = Command::detokenize(detokenizer)?;
                if eod != Command::EndOfData {
                    return Err(D::Error::message("expected an END_OF_DATA token"));
                }

                let status = Vec::<MethodStatus>::detokenize(detokenizer)?;
                let Some(status) = status.first().cloned() else {
                    return Err(D::Error::message("received empty method status list"));
                };

                match status {
                    MethodStatus::Success => Ok(Self(Ok(param_list))),
                    failure => Ok(Self(Err(failure))),
                }
            }
            Err(param_err) => {
                // Consume the token stream all the way to the EOD command, if any.
                let eod_result = detokenizer.detokenize_until(|detokenizer| match Command::detokenize(detokenizer) {
                    result @ Ok(Command::EndOfData) => result,
                    Ok(_) => Err(D::Error::message("expected an END_OF_DATA token")),
                    result => result,
                });

                if eod_result.is_err() {
                    return Err(param_err);
                }

                let status = Vec::<MethodStatus>::detokenize(detokenizer)?;
                let Some(status) = status.first().cloned() else {
                    return Err(D::Error::message("received empty method status list"));
                };

                match status {
                    MethodStatus::Success => Err(param_err),
                    failure => Ok(Self(Err(failure))),
                }
            }
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
    struct ParamList {
        a: u8,
        b: Option<u8>,
    }

    #[rstest]
    #[case::without_optional(MethodResult(Ok(ParamList{a: 1, b: None})), &[0xF0, 0x01, 0xF1, 0xF9, 0xF0, 0x00, 0x00, 0x00, 0xF1])]
    #[case::with_optional(MethodResult(Ok(ParamList{a: 1, b: Some(2)})), &[0xF0, 0x01, 0xF2, 0x00, 0x02, 0xF3, 0xF1, 0xF9, 0xF0, 0x00, 0x00, 0x00, 0xF1])]
    #[case::fail(MethodResult(Err(MethodStatus::Fail)), &[0xF0, 0xF1, 0xF9, 0xF0, 0x3F, 0x00, 0x00, 0xF1])]
    fn tokenize(#[case] value: MethodResult<ParamList>, #[case] bytes: &[u8]) {
        assert_eq!(value.to_tokens().unwrap(), bytes);
        assert_eq!(<MethodResult<ParamList>>::from_tokens(bytes).unwrap(), value);
    }

    #[rstest]
    #[case::valid_param_list_fail(Ok(MethodResult(Err(MethodStatus::Fail))), &[0xF0, 0x01, 0xF1, 0xF9, 0xF0, 0x3F, 0x00, 0x00, 0xF1])]
    #[case::empty_param_list_success(Err(TokenError::Custom("mandatory field a missing".into())), &[0xF0, 0xF1, 0xF9, 0xF0, 0x00, 0x00, 0x00, 0xF1])]
    #[case::invalid_param_list(Err(TokenError::CanNotConvert{ from: "control", to: "u8" }), &[0xF0, 0xFF, 0xF1, 0xF9, 0xF0, 0x00, 0x00, 0x00, 0xF1])]
    #[case::failed_and_eod_not_found(Err(TokenError::CanNotConvert{ from: "control", to: "u8" }), &[0xF0, 0xFF, 0xF1, 0xF0, 0x00, 0x00, 0x00, 0xF1])]
    fn detokenize_edge_cases(#[case] value: Result<MethodResult<ParamList>, TokenError>, #[case] bytes: &[u8]) {
        assert_eq!(<MethodResult<ParamList>>::from_tokens(bytes), value);
    }
}
