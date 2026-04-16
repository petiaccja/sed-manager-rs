use sed_packet::Uid;
use sed_packet::token::{Command, Detokenize, Detokenizer, MessageError as _, Tokenize, Tokenizer};

use crate::methods::{CloseSession, MethodStatus, PropertiesMethod, StartSession, SyncSession};
use crate::preconfig::core::shared::invoking_id::SESSION_MANAGER;
use crate::preconfig::core::shared::sm_method_id::{CLOSE_SESSION, PROPERTIES, START_SESSION, SYNC_SESSION};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodCall<Parameters> {
    pub invoking_id: Uid,
    pub method_id: Uid,
    pub parameters: Parameters,
    pub status: MethodStatus,
}

impl<Parameters> Tokenize for MethodCall<Parameters>
where
    Parameters: Tokenize,
{
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        Command::Call.tokenize(tokenizer)?;
        self.invoking_id.tokenize(tokenizer)?;
        self.method_id.tokenize(tokenizer)?;
        self.parameters.tokenize(tokenizer)?;
        Command::EndOfData.tokenize(tokenizer)?;
        vec![self.status, MethodStatus::Success, MethodStatus::Success].tokenize(tokenizer)?;
        Ok(())
    }
}

impl<Parameters> Detokenize for MethodCall<Parameters>
where
    Parameters: Detokenize,
{
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        let call_command = Command::detokenize(detokenizer)?;
        match call_command {
            Command::Call => (),
            _ => return Err(D::Error::message("expected a CALL token")),
        }
        let invoking_id = Uid::detokenize(detokenizer)?;
        let method_id = Uid::detokenize(detokenizer)?;
        let parameters = Parameters::detokenize(detokenizer)?;
        let eod_command = Command::detokenize(detokenizer)?;
        match eod_command {
            Command::EndOfData => (),
            _ => return Err(D::Error::message("expected an END_OF_DATA token")),
        }
        let status = Vec::<MethodStatus>::detokenize(detokenizer)?;
        let Some(status) = status.first().cloned() else {
            return Err(D::Error::message("received empty method status list"));
        };
        Ok(Self { invoking_id, method_id, parameters, status })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MgmtMethodCall {
    pub params: MgmtMethodCallParams,
    pub status: MethodStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MgmtMethodCallParams {
    StartSession(StartSession),
    SyncSession(SyncSession),
    CloseSession(CloseSession),
    Properties(PropertiesMethod),
}

impl Tokenize for MgmtMethodCall {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        Command::Call.tokenize(tokenizer)?;
        SESSION_MANAGER.tokenize(tokenizer)?;
        match &self.params {
            MgmtMethodCallParams::StartSession(_) => START_SESSION.tokenize(tokenizer)?,
            MgmtMethodCallParams::SyncSession(_) => SYNC_SESSION.tokenize(tokenizer)?,
            MgmtMethodCallParams::CloseSession(_) => CLOSE_SESSION.tokenize(tokenizer)?,
            MgmtMethodCallParams::Properties(_) => PROPERTIES.tokenize(tokenizer)?,
        }
        match &self.params {
            MgmtMethodCallParams::StartSession(params) => params.tokenize(tokenizer)?,
            MgmtMethodCallParams::SyncSession(params) => params.tokenize(tokenizer)?,
            MgmtMethodCallParams::CloseSession(params) => params.tokenize(tokenizer)?,
            MgmtMethodCallParams::Properties(params) => params.tokenize(tokenizer)?,
        }
        Command::EndOfData.tokenize(tokenizer)?;
        vec![self.status, MethodStatus::Success, MethodStatus::Success].tokenize(tokenizer)?;
        Ok(())
    }
}

impl Detokenize for MgmtMethodCall {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        let call_command = Command::detokenize(detokenizer)?;
        match call_command {
            Command::Call => (),
            _ => return Err(D::Error::message("expected a CALL token")),
        }
        let invoking_id = Uid::detokenize(detokenizer)?;
        if invoking_id != SESSION_MANAGER {
            return Err(D::Error::message("expected a SMUID as invoking ID"));
        }
        let method_id = Uid::detokenize(detokenizer)?;
        let params = match method_id {
            START_SESSION => MgmtMethodCallParams::StartSession(StartSession::detokenize(detokenizer)?),
            SYNC_SESSION => MgmtMethodCallParams::SyncSession(SyncSession::detokenize(detokenizer)?),
            CLOSE_SESSION => MgmtMethodCallParams::CloseSession(CloseSession::detokenize(detokenizer)?),
            PROPERTIES => MgmtMethodCallParams::Properties(PropertiesMethod::detokenize(detokenizer)?),
            _ => return Err(D::Error::message(format!("unrecognized SM method {}", method_id))),
        };
        let eod_command = Command::detokenize(detokenizer)?;
        match eod_command {
            Command::EndOfData => (),
            _ => return Err(D::Error::message("expected an END_OF_DATA token")),
        }
        let status = Vec::<MethodStatus>::detokenize(detokenizer)?;
        let Some(status) = status.first().cloned() else {
            return Err(D::Error::message("received empty method status list"));
        };
        Ok(Self { params, status })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod tests {
        use super::*;

        use rstest::rstest;
        use sed_packet::token::{FromTokens as _, ToTokens as _};
        use sed_spec_macros::{DetokenizeStruct, TokenizeStruct};

        #[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
        struct Parameters {
            a: u8,
            b: Option<u8>,
        }

        #[test]
        fn tokenize() {
            let value = MethodCall {
                invoking_id: Uid::new(0x56),
                method_id: Uid::new(0x78),
                parameters: Parameters { a: 1, b: None },
                status: MethodStatus::Fail,
            };

            #[rustfmt::skip]
            let bytes = &[
                0xF8, // CALL
                0b1010_1000, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x56, // Invoking ID
                0b1010_1000, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x78, // Method ID
                0xF0, 0x01, 0xF1, // Parameters
                0xF9, // EOD
                0xF0, 0x3F, 0x00, 0x00, 0xF1, // Status
            ];

            assert_eq!(value.to_tokens().unwrap(), bytes);
            assert_eq!(<MethodCall<Parameters>>::from_tokens(bytes).unwrap(), value);
        }

        #[rstest]
        // Missing CALL token
        #[case(
            &[
                0xF9, // NOT CALL
                0b1010_1000, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x56, // Invoking ID
                0b1010_1000, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x78, // Method ID
                0xF0, 0x01, 0xF1, // Parameters
                0xF9, // EOD
                0xF0, 0x3F, 0x00, 0x00, 0xF1, // Status
            ]
        )]
        // Missing EOD token
        #[case(
            &[
                0xF8, // CALL
                0b1010_1000, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x56, // Invoking ID
                0b1010_1000, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x78, // Method ID
                0xF0, 0x01, 0xF1, // Parameters
                0xF0, 0x3F, 0x00, 0x00, 0xF1, // Status
            ]
        )]
        // Empty status list
        #[case(
            &[
                0xF8, // CALL
                0b1010_1000, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x56, // Invoking ID
                0b1010_1000, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x78, // Method ID
                0xF0, 0x01, 0xF1, // Parameters
                0xF9, // EOD
                0xF0, 0xF1, // Status
            ]
        )]
        fn detokenize_edge_cases(#[case] bytes: &[u8]) {
            assert!(<MethodCall<Parameters>>::from_tokens(bytes).is_err());
        }

        #[test]
        fn sm_tokenize() {
            let value = MgmtMethodCall {
                params: MgmtMethodCallParams::CloseSession(CloseSession {
                    remote_session_number: 1,
                    local_session_number: 2,
                }),
                status: MethodStatus::Fail,
            };

            #[rustfmt::skip]
            let bytes = &[
                0xF8, // CALL
                0b1010_1000, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, // Invoking ID
                0b1010_1000, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0x06, // Method ID
                0xF0, 0x01, 0x02, 0xF1, // Parameters
                0xF9, // EOD
                0xF0, 0x3F, 0x00, 0x00, 0xF1, // Status
            ];

            assert_eq!(value.to_tokens().unwrap(), bytes);
            assert_eq!(MgmtMethodCall::from_tokens(bytes).unwrap(), value);
        }

        #[rstest]
        // Missing CALL token
        #[case(
            &[
                0xF9, // CALL
                0b1010_1000, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, // Invoking ID
                0b1010_1000, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0x06, // Method ID
                0xF0, 0x01, 0x02, 0xF1, // Parameters
                0xF9, // EOD
                0xF0, 0x3F, 0x00, 0x00, 0xF1, // Status
            ]
        )]
        // Missing EOD token
        #[case(
            &[
                0xF8, // CALL
                0b1010_1000, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, // Invoking ID
                0b1010_1000, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0x06, // Method ID
                0xF0, 0x01, 0x02, 0xF1, // Parameters
                0xF0, 0x3F, 0x00, 0x00, 0xF1, // Status
            ]
        )]
        // Empty status list
        #[case(
            &[
                0xF8, // CALL
                0b1010_1000, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, // Invoking ID
                0b1010_1000, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0x06, // Method ID
                0xF0, 0x01, 0x02, 0xF1, // Parameters
                0xF9, // EOD
                0xF0, 0xF1, // Status
            ]
        )]
        // Not SMUID
        #[case(
            &[
                0xF8, // CALL
                0b1010_1000, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFE, // Invoking ID
                0b1010_1000, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0x06, // Method ID
                0xF0, 0x01, 0x02, 0xF1, // Parameters
                0xF9, // EOD
                0xF0, 0x3F, 0x00, 0x00, 0xF1, // Status
            ]
        )]
        // Unknown method ID
        #[case(
            &[
                0xF8, // CALL
                0b1010_1000, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, // Invoking ID
                0b1010_1000, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xAF, // Method ID
                0xF0, 0x01, 0x02, 0xF1, // Parameters
                0xF9, // EOD
                0xF0, 0x3F, 0x00, 0x00, 0xF1, // Status
            ]
        )]
        fn sm_detokenize_edge_cases(#[case] bytes: &[u8]) {
            assert!(<MethodCall<Parameters>>::from_tokens(bytes).is_err());
        }
    }
}
