use crate::token::{Detokenize, Detokenizer, MessageError as _, Tokenize, Tokenizer};

impl Tokenize for bool {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        (*self as u8).tokenize(tokenizer)
    }
}

impl Detokenize for bool {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        match u8::detokenize(detokenizer)? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(D::Error::message("invalid boolean value")),
        }
    }
}
