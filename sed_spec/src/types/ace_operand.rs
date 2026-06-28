use sed_packet::{
    Named,
    token::{Detokenize, Detokenizer, MessageError, Tokenize, Tokenizer},
};

use crate::{
    objects::AuthorityRef,
    types::{Type, boolean_op::BooleanOp},
};

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum AceOperand {
    Authority(AuthorityRef),
    BooleanOp(BooleanOp),
}

impl From<AuthorityRef> for AceOperand {
    fn from(value: AuthorityRef) -> Self {
        Self::Authority(value)
    }
}

impl From<BooleanOp> for AceOperand {
    fn from(value: BooleanOp) -> Self {
        Self::BooleanOp(value)
    }
}
impl From<&AuthorityRef> for AceOperand {
    fn from(value: &AuthorityRef) -> Self {
        Self::Authority(*value)
    }
}

impl From<&BooleanOp> for AceOperand {
    fn from(value: &BooleanOp) -> Self {
        Self::BooleanOp(*value)
    }
}

impl Tokenize for AceOperand {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        match self {
            Self::Authority(object_ref) => {
                Named { name: AuthorityRef::UID.to_half().to_be_bytes(), value: object_ref }.tokenize(tokenizer)
            }
            Self::BooleanOp(boolean_op) => {
                Named { name: BooleanOp::UID.to_half().to_be_bytes(), value: boolean_op }.tokenize(tokenizer)
            }
        }
    }
}

impl Detokenize for AceOperand {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        let (_, value) = detokenizer.detokenize_named(
            |detokenizer| <[u8; 4]>::detokenize(detokenizer).map(|bytes| u32::from_be_bytes(bytes)),
            |detokenizer, name| match *name {
                x if x == AuthorityRef::UID.to_half() => {
                    AuthorityRef::detokenize(detokenizer).map(|auth_ref| AceOperand::Authority(auth_ref))
                }
                x if x == BooleanOp::UID.to_half() => {
                    BooleanOp::detokenize(detokenizer).map(|bool_op| AceOperand::BooleanOp(bool_op))
                }
                _ => Err(D::Error::message("invalid type alternative for ACE operand")),
            },
        )?;
        Ok(value)
    }
}
