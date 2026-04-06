use crate::token::{Command, Detokenize, Detokenizer, Tokenize, Tokenizer};

impl Tokenize for Command {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        tokenizer.tokenize_command(*self)
    }
}

impl Detokenize for Command {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        detokenizer.detokenize_command()
    }
}
