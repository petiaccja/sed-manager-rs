use crate::token::{Detokenize, Detokenizer};

pub struct Ignore;

impl Detokenize for Ignore {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        detokenizer.ignore(16).map(|_| Ignore)
    }
}
