use smallvec::SmallVec;

use crate::token::{Detokenize, Detokenizer, MessageError, Tokenize, Tokenizer};

pub type MaxBytes<const N: usize> = SmallVec<[u8; N]>;

impl<const N: usize> Tokenize for SmallVec<[u8; N]> {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        tokenizer.tokenize_bytes(self.as_slice())
    }
}

impl<const N: usize> Detokenize for SmallVec<[u8; N]> {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        let bytes = detokenizer.detokenize_bytes()?;
        bytes.try_into().map_err(|_| D::Error::message("unexpected array length"))
    }
}
