use num_enum::{FromPrimitive, IntoPrimitive};
use sed_packet::token::{Detokenize, Detokenizer, Tokenize, Tokenizer};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, IntoPrimitive)]
pub enum LifeCycleState {
    Issued = 0,
    IssuedDisabled = 1,
    IssuedFrozen = 2,
    IssuedDisabledFrozen = 3,
    IssuedFailed = 4,
    ManufacturedInactive = 8,
    Manufactured = 9,
    ManufacturedDisabled = 10,
    ManufacturedFrozen = 11,
    ManufacturedDisabledFrozen = 12,
    ManufacturedFailed = 13,
    #[num_enum(catch_all)]
    Unknown(u8),
}

impl Tokenize for LifeCycleState {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        u8::from(*self).tokenize(tokenizer)
    }
}

impl Detokenize for LifeCycleState {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        Ok(Self::from(u8::detokenize(detokenizer)?))
    }
}
