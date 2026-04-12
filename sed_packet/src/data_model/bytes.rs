use crate::token::{Detokenize, Detokenizer, Tokenize, Tokenizer};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Bytes(pub Vec<u8>);

impl Tokenize for Bytes {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        tokenizer.tokenize_bytes(self.0.as_slice())
    }
}

impl Detokenize for Bytes {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        let bytes = detokenizer.detokenize_bytes()?;
        Ok(Bytes(bytes))
    }
}
