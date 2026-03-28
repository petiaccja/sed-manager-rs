use crate::token::{Detokenize, Detokenizer, Tokenize, Tokenizer};

macro_rules! impl_tokenize {
    ($type:ty, $fn:ident) => {
        impl Tokenize for $type {
            fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
                tokenizer.$fn(*self)
            }
        }
    };
}

macro_rules! impl_detokenize {
    ($type:ty, $fn:ident) => {
        impl Detokenize for $type {
            fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
                detokenizer.$fn()
            }
        }
    };
}

impl_tokenize!(i8, tokenize_i8);
impl_tokenize!(i16, tokenize_i16);
impl_tokenize!(i32, tokenize_i32);
impl_tokenize!(i64, tokenize_i64);
impl_tokenize!(u8, tokenize_u8);
impl_tokenize!(u16, tokenize_u16);
impl_tokenize!(u32, tokenize_u32);
impl_tokenize!(u64, tokenize_u64);

impl_detokenize!(i8, detokenize_i8);
impl_detokenize!(i16, detokenize_i16);
impl_detokenize!(i32, detokenize_i32);
impl_detokenize!(i64, detokenize_i64);
impl_detokenize!(u8, detokenize_u8);
impl_detokenize!(u16, detokenize_u16);
impl_detokenize!(u32, detokenize_u32);
impl_detokenize!(u64, detokenize_u64);
