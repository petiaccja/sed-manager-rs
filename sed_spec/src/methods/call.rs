use sed_packet::token::{Command, Detokenize, Detokenizer, MessageError as _, Tokenize, Tokenizer};
use sed_packet::{TableRef, Uid};

use crate::methods::{
    Activate, Authenticate, CloseSession, GenKey, Get, GetAcl, MethodParam, MethodStatus, NextUntyped,
    PropertiesMethod, Random, Revert, RevertSp, SessionMethodParam as _, SetBytes, SetObject, StartSession,
    SyncSession,
};
use crate::objects::{Ace, Authority, CPin, KAes256, LockingRange, MbrControl, MethodRef, SecurityProvider, TableDesc};
use crate::preconfig::core::shared::invoking_id::SESSION_MANAGER;
use crate::preconfig::core::shared::{method_id, table_id};

//------------------------------------------------------------------------------
// Generic method call
//------------------------------------------------------------------------------

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

//------------------------------------------------------------------------------
// Session manager method call
//------------------------------------------------------------------------------

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
            MgmtMethodCallParams::StartSession(params) => {
                params.method_id().tokenize(tokenizer)?;
                params.tokenize(tokenizer)?;
            }
            MgmtMethodCallParams::SyncSession(params) => {
                params.method_id().tokenize(tokenizer)?;
                params.tokenize(tokenizer)?;
            }
            MgmtMethodCallParams::CloseSession(params) => {
                params.method_id().tokenize(tokenizer)?;
                params.tokenize(tokenizer)?;
            }
            MgmtMethodCallParams::Properties(params) => {
                params.method_id().tokenize(tokenizer)?;
                params.tokenize(tokenizer)?;
            }
        };
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
            StartSession::METHOD_ID => MgmtMethodCallParams::StartSession(<_>::detokenize(detokenizer)?),
            SyncSession::METHOD_ID => MgmtMethodCallParams::SyncSession(<_>::detokenize(detokenizer)?),
            CloseSession::METHOD_ID => MgmtMethodCallParams::CloseSession(<_>::detokenize(detokenizer)?),
            PropertiesMethod::METHOD_ID => MgmtMethodCallParams::Properties(<_>::detokenize(detokenizer)?),
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

//------------------------------------------------------------------------------
// Session method call
//------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionMethodCall {
    pub invoking_id: Uid,
    pub params: SessionMethodCallParams,
    pub status: MethodStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionMethodCallParams {
    Activate(Activate),
    Authenticate(Authenticate),
    Next(NextUntyped),
    GetAcl(GetAcl),
    GenKey(GenKey),
    Revert(Revert),
    RevertSp(RevertSp),
    Random(Random),
    Get(Get),
    SetAce(SetObject<Ace>),
    SetAuthority(SetObject<Authority>),
    SetCPin(SetObject<CPin>),
    SetKAes256(SetObject<KAes256>),
    SetLockingRange(SetObject<LockingRange>),
    SetMbrControl(SetObject<MbrControl>),
    SetSecurityProvider(SetObject<SecurityProvider>),
    SetTableDesc(SetObject<TableDesc>),
    SetBytes(SetBytes),
}

impl Tokenize for SessionMethodCall {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        Command::Call.tokenize(tokenizer)?;
        self.invoking_id.tokenize(tokenizer)?;
        match &self.params {
            SessionMethodCallParams::Activate(params) => {
                params.method_id().tokenize(tokenizer)?;
                params.tokenize(tokenizer)?;
            }
            SessionMethodCallParams::Authenticate(params) => {
                params.method_id().tokenize(tokenizer)?;
                params.tokenize(tokenizer)?;
            }
            SessionMethodCallParams::Next(params) => {
                params.method_id().tokenize(tokenizer)?;
                params.tokenize(tokenizer)?;
            }
            SessionMethodCallParams::GetAcl(params) => {
                params.method_id().tokenize(tokenizer)?;
                params.tokenize(tokenizer)?;
            }
            SessionMethodCallParams::GenKey(params) => {
                params.method_id().tokenize(tokenizer)?;
                params.tokenize(tokenizer)?;
            }
            SessionMethodCallParams::Revert(params) => {
                params.method_id().tokenize(tokenizer)?;
                params.tokenize(tokenizer)?;
            }
            SessionMethodCallParams::RevertSp(params) => {
                params.method_id().tokenize(tokenizer)?;
                params.tokenize(tokenizer)?;
            }
            SessionMethodCallParams::Random(params) => {
                params.method_id().tokenize(tokenizer)?;
                params.tokenize(tokenizer)?;
            }
            SessionMethodCallParams::Get(params) => {
                params.method_id().tokenize(tokenizer)?;
                params.tokenize(tokenizer)?;
            }
            SessionMethodCallParams::SetAce(params) => {
                params.method_id().tokenize(tokenizer)?;
                params.tokenize(tokenizer)?;
            }
            SessionMethodCallParams::SetAuthority(params) => {
                params.method_id().tokenize(tokenizer)?;
                params.tokenize(tokenizer)?;
            }
            SessionMethodCallParams::SetCPin(params) => {
                params.method_id().tokenize(tokenizer)?;
                params.tokenize(tokenizer)?;
            }
            SessionMethodCallParams::SetKAes256(params) => {
                params.method_id().tokenize(tokenizer)?;
                params.tokenize(tokenizer)?;
            }
            SessionMethodCallParams::SetLockingRange(params) => {
                params.method_id().tokenize(tokenizer)?;
                params.tokenize(tokenizer)?;
            }
            SessionMethodCallParams::SetMbrControl(params) => {
                params.method_id().tokenize(tokenizer)?;
                params.tokenize(tokenizer)?;
            }
            SessionMethodCallParams::SetSecurityProvider(params) => {
                params.method_id().tokenize(tokenizer)?;
                params.tokenize(tokenizer)?;
            }
            SessionMethodCallParams::SetTableDesc(params) => {
                params.method_id().tokenize(tokenizer)?;
                params.tokenize(tokenizer)?;
            }
            SessionMethodCallParams::SetBytes(params) => {
                params.method_id().tokenize(tokenizer)?;
                params.tokenize(tokenizer)?;
            }
        };
        Command::EndOfData.tokenize(tokenizer)?;
        vec![self.status, MethodStatus::Success, MethodStatus::Success].tokenize(tokenizer)?;
        Ok(())
    }
}

impl Detokenize for SessionMethodCall {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        let call_command = Command::detokenize(detokenizer)?;
        match call_command {
            Command::Call => (),
            _ => return Err(D::Error::message("expected a CALL token")),
        }
        let invoking_id = Uid::detokenize(detokenizer)?;
        let method_id = MethodRef::detokenize(detokenizer)?;
        let invoking_table = TableRef::containing_table(invoking_id).or(TableRef::try_from(invoking_id).ok());
        let params = match method_id {
            Activate::METHOD_ID => SessionMethodCallParams::Activate(<_>::detokenize(detokenizer)?),
            Authenticate::METHOD_ID => SessionMethodCallParams::Authenticate(<_>::detokenize(detokenizer)?),
            NextUntyped::METHOD_ID => SessionMethodCallParams::Next(<_>::detokenize(detokenizer)?),
            GetAcl::METHOD_ID => SessionMethodCallParams::GetAcl(<_>::detokenize(detokenizer)?),
            GenKey::METHOD_ID => SessionMethodCallParams::GenKey(<_>::detokenize(detokenizer)?),
            Revert::METHOD_ID => SessionMethodCallParams::Revert(<_>::detokenize(detokenizer)?),
            RevertSp::METHOD_ID => SessionMethodCallParams::RevertSp(<_>::detokenize(detokenizer)?),
            Random::METHOD_ID => SessionMethodCallParams::Random(<_>::detokenize(detokenizer)?),
            Get::METHOD_ID => SessionMethodCallParams::Get(<_>::detokenize(detokenizer)?),
            method_id if method_id == method_id::SET => match invoking_table {
                Some(table_id::ACE) => SessionMethodCallParams::SetAce(<_>::detokenize(detokenizer)?),
                Some(table_id::AUTHORITY) => SessionMethodCallParams::SetAuthority(<_>::detokenize(detokenizer)?),
                Some(table_id::C_PIN) => SessionMethodCallParams::SetCPin(<_>::detokenize(detokenizer)?),
                Some(table_id::K_AES_256) => SessionMethodCallParams::SetKAes256(<_>::detokenize(detokenizer)?),
                Some(table_id::LOCKING) => SessionMethodCallParams::SetLockingRange(<_>::detokenize(detokenizer)?),
                Some(table_id::MBR_CONTROL) => SessionMethodCallParams::SetMbrControl(<_>::detokenize(detokenizer)?),
                Some(table_id::SP) => SessionMethodCallParams::SetSecurityProvider(<_>::detokenize(detokenizer)?),
                Some(table_id::TABLE) => SessionMethodCallParams::SetTableDesc(<_>::detokenize(detokenizer)?),
                Some(table_id::MBR) => SessionMethodCallParams::SetBytes(<_>::detokenize(detokenizer)?),
                Some(table) => return Err(D::Error::message(format!("Set method: unrecognized table: {}", table))),
                None => {
                    return Err(D::Error::message(format!("Set method: invoking_id ({}) is not a table", invoking_id)));
                }
            },
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
        Ok(Self { invoking_id, params, status })
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
