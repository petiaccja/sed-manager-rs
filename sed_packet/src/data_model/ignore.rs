//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::token_stream::{Detokenize, Detokenizer};

pub struct Ignore;

impl Detokenize for Ignore {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        detokenizer.ignore(16).map(|_| Ignore)
    }
}
