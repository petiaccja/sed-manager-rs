use num_enum::{FromPrimitive, IntoPrimitive};

use sed_packet::token::{Detokenize, Detokenizer, Tokenize, Tokenizer};

#[allow(non_camel_case_types)]
#[derive(PartialEq, Eq, Clone, Copy, Debug, FromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum MessagingType {
    None = 0,
    HMAC_SHA_256 = 1,
    HMAC_SHA_384 = 2,
    HMAC_SHA_512 = 3,
    RSASSA_PKCS1_v1_5_1024 = 4,
    RSASSA_PKCS1_v1_5_2048 = 5,
    RSASSA_PKCS1_v1_5_3072 = 6,
    RSASSA_PSS_1024 = 7,
    RSASSA_PSS_2048 = 8,
    RSASSA_PSS_3072 = 9,
    ECDSA_256_SHA_256 = 10,
    ECDSA_384_SHA_384 = 11,
    ECDSA_512_SHA_512 = 12,
    CMAC_128_with_128_bit_MAC = 13,
    CMAC_256_with_128_bit_MAC = 14,
    GMAC_128_with_128_bit_MAC_and_96_bit_IV = 15,
    GMAC_256_with_128_bit_MAC_and_96_bit_IV = 16,
    AES_CBC_128 = 64,
    AES_CBC_256 = 65,
    AES_CBC_128_with_HMAC_SHA_256 = 129,
    AES_CBC_256_with_HMAC_SHA_256 = 130,
    AES_CBC_256_with_HMAC_SHA_384 = 131,
    AES_CBC_256_with_HMAC_SHA_512 = 132,
    AES_CCM_128_with_128_bit_MAC = 133,
    AES_CCM_256_with_128_bit_MAC = 134,
    AES_GCM_128_with_128_bit_MAC = 135,
    AES_GCM_256_with_128_bit_MAC = 136,
    #[num_enum(catch_all)]
    Unknown(u8) = 255,
}

impl Tokenize for MessagingType {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        u8::from(*self).tokenize(tokenizer)
    }
}

impl Detokenize for MessagingType {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        Ok(Self::from(u8::detokenize(detokenizer)?))
    }
}
