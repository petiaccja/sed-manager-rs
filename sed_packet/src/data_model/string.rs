use crate::token::{Detokenize, Detokenizer, MessageError, Tokenize, Tokenizer};

impl Tokenize for String {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        tokenizer.tokenize_bytes(self.as_bytes())
    }
}

impl Tokenize for &str {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        tokenizer.tokenize_bytes(self.as_bytes())
    }
}

impl Detokenize for String {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        let bytes = detokenizer.detokenize_bytes()?;
        bytes.try_into().map_err(|_| D::Error::message("invalid UTF-8 characters"))
    }
}
