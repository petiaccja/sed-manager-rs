use sed_packet::Uid;
use sed_packet::token::{Command, Detokenize, Detokenizer, MessageError as _, Tokenize, Tokenizer};

use crate::methods::MethodStatus;

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
