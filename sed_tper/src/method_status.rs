use num_enum::{IntoPrimitive, TryFromPrimitive};
use sed_packet::token::{Detokenize, Detokenizer, MessageError, Tokenize, Tokenizer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum MethodStatus {
    #[error("Success")]
    Success = 0x00,
    #[error("Not authorized")]
    NotAuthorized = 0x01,
    #[error("TCG Obsolete status code #0")]
    Obsolete0 = 0x02,
    #[error("Security provider is busy")]
    SPBusy = 0x03,
    #[error("Security provider has failed")]
    SPFailed = 0x04,
    #[error("Security provider is disabled")]
    SPDisabled = 0x05,
    #[error("Security provider is frozen")]
    SPFrozen = 0x06,
    #[error("No more sessions are available")]
    NoSessionsAvailable = 0x07,
    #[error("Uniqueness conflict")]
    UniquenessConflict = 0x08,
    #[error("No more space is available")]
    InsufficientSpace = 0x09,
    #[error("No more rows are available")]
    InsufficientRows = 0x0A,
    #[error("Invalid parameter was provided to an RPC method call")]
    InvalidParameter = 0x0C,
    #[error("TCG Obsolete status code #1")]
    Obsolete1 = 0x0D,
    #[error("TCG Obsolete status code #2")]
    Obsolete2 = 0x0E,
    #[error("TPer malfunction")]
    TPerMalfunction = 0x0F,
    #[error("Transaction failed")]
    TransactionFailure = 0x10,
    #[error("Response overflow")]
    ResponseOverflow = 0x11,
    #[error("The authority is locked out")]
    AuthorityLockedOut = 0x12,
    #[error("RPC method call failed (cause unspecified)")]
    Fail = 0x3F,
}

impl Tokenize for MethodStatus {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        u8::from(*self).tokenize(tokenizer)
    }
}

impl Detokenize for MethodStatus {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        u8::detokenize(detokenizer)
            .map(|value| Self::try_from(value).map_err(|_| D::Error::message("invalid method status value")))
            .flatten()
    }
}
