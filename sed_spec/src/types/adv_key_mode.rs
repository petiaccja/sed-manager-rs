use num_enum::{FromPrimitive, IntoPrimitive};
use sed_packet::token::{Detokenize, Detokenizer, Tokenize, Tokenizer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum AdvKeyMode {
    WaitForAdvKeyReq = 0,
    AutoAdvanceKeys = 1,
    #[num_enum(catch_all)]
    Unknown(u8) = 7,
}

impl Tokenize for AdvKeyMode {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        u8::from(*self).tokenize(tokenizer)
    }
}

impl Detokenize for AdvKeyMode {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        Ok(Self::from(u8::detokenize(detokenizer)?))
    }
}
