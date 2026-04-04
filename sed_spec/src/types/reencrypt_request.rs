use num_enum::{FromPrimitive, IntoPrimitive};
use sed_packet::token::{Detokenize, Detokenizer, Tokenize, Tokenizer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum ReencryptRequest {
    // This is not empty as a mistake.
    // The values are missing from the core specification.
    #[num_enum(catch_all)]
    Unknown(u8) = 16,
}

impl Tokenize for ReencryptRequest {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        u8::from(*self).tokenize(tokenizer)
    }
}

impl Detokenize for ReencryptRequest {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        Ok(Self::from(u8::detokenize(detokenizer)?))
    }
}
