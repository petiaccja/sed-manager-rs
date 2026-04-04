use num_enum::{FromPrimitive, IntoPrimitive};

use sed_packet::token::{Detokenize, Detokenizer, Tokenize, Tokenizer};

#[derive(PartialEq, Eq, Clone, Copy, Debug, FromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum HashProtocol {
    None = 0,
    SHA1 = 1,
    SHA256 = 2,
    SHA384 = 3,
    SHA512 = 4,
    #[num_enum(catch_all)]
    Unknown(u8),
}

impl Tokenize for HashProtocol {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        u8::from(*self).tokenize(tokenizer)
    }
}

impl Detokenize for HashProtocol {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        Ok(Self::from(u8::detokenize(detokenizer)?))
    }
}
