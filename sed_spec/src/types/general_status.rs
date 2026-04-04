use num_enum::{FromPrimitive, IntoPrimitive};
use sed_packet::token::{Detokenize, Detokenizer, Tokenize, Tokenizer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum GeneralStatus {
    None = 0,
    PendingTPerError = 1,
    ActiveTPerError = 2,
    ActivePauseRequest = 3,
    PendingPauseRequested = 4,
    PendingResetStopDetected = 5,
    KeyError = 6,
    WaitAvailableKeys = 32,
    WaitForTPerResources = 33,
    ActiveResetStopDetected = 34,
    #[num_enum(catch_all)]
    Unknown(u8) = 63,
}

impl Tokenize for GeneralStatus {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        u8::from(*self).tokenize(tokenizer)
    }
}

impl Detokenize for GeneralStatus {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        Ok(Self::from(u8::detokenize(detokenizer)?))
    }
}
