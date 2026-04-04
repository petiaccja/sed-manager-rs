use num_enum::{FromPrimitive, IntoPrimitive};

use sed_packet::token::{Detokenize, Detokenizer, Tokenize, Tokenizer};

#[allow(non_camel_case_types)]
#[derive(PartialEq, Eq, Clone, Copy, Debug, FromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum SymmetricModeMedia {
    ECB = 0,
    CBC = 1,
    CFB = 2,
    OFB = 3,
    GCM = 4,
    CTR = 5,
    CCM = 6,
    XTS = 7,
    LRW = 8,
    EME = 9,
    CMC = 10,
    XEX = 11,
    MediaEncryption = 23,
    #[num_enum(catch_all)]
    Unknown(u8) = 22,
}

impl Tokenize for SymmetricModeMedia {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        u8::from(*self).tokenize(tokenizer)
    }
}

impl Detokenize for SymmetricModeMedia {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        Ok(Self::from(u8::detokenize(detokenizer)?))
    }
}
