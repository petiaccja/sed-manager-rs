use num_enum::{FromPrimitive, IntoPrimitive};

use sed_packet::token::{Detokenize, Detokenizer, Tokenize, Tokenizer};

#[derive(PartialEq, Eq, Clone, Copy, Debug, FromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum AuthMethod {
    None = 0,
    Password = 1,
    Exchange = 2,
    Sign = 3,
    SymK = 4,
    HMAC = 5,
    TPerSign = 6,
    TPerExchange = 7,
    #[num_enum(catch_all)]
    Unknown(u8),
}

impl Tokenize for AuthMethod {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        u8::from(*self).tokenize(tokenizer)
    }
}

impl Detokenize for AuthMethod {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        Ok(Self::from(u8::detokenize(detokenizer)?))
    }
}
