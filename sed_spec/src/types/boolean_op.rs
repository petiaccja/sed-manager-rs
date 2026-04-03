use num_enum::{IntoPrimitive, TryFromPrimitive};

use sed_packet::token::{Detokenize, Detokenizer, MessageError, Tokenize, Tokenizer};

use crate::{objects::TypeRef, types::r#type::Type};

#[derive(PartialEq, Eq, Clone, Debug, Copy, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum BooleanOp {
    And = 0,
    Or = 1,
    Not = 2,
}

impl Type for BooleanOp {
    const UID: TypeRef = TypeRef::new_unchecked(0x0000_0005_0000_040E);
}

impl Tokenize for BooleanOp {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        u8::from(*self).tokenize(tokenizer)
    }
}

impl Detokenize for BooleanOp {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        Self::try_from(u8::detokenize(detokenizer)?).map_err(|_| D::Error::message("invalid enumeration value"))
    }
}
