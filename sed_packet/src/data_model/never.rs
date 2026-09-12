//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::convert::Infallible;

use crate::token::{Detokenize, Detokenizer, Tokenize, Tokenizer};

impl Tokenize for Infallible {
    fn tokenize<T: Tokenizer>(&self, _tokenizer: &mut T) -> Result<(), T::Error> {
        unreachable!("the never/infallible type cannot be instantiated")
    }
}

impl Detokenize for Infallible {
    fn detokenize<D: Detokenizer>(_detokenizer: &mut D) -> Result<Self, D::Error> {
        unreachable!("the never/infallible type cannot be instantiated")
    }
}
