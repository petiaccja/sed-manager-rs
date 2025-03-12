use sed_manager_macros::AlternativeType;

use super::{AuthorityRef, BooleanOp, Bytes32, Bytes64};

#[derive(AlternativeType, PartialEq, Eq, Clone, Debug)]
pub enum Key256 {
    Bytes32(Bytes32),
    Bytes64(Bytes64),
}

#[derive(AlternativeType, PartialEq, Eq, Clone, Debug)]
pub enum ACEOperand {
    Authority(AuthorityRef),
    BooleanOp(BooleanOp),
}

impl From<AuthorityRef> for ACEOperand {
    fn from(value: AuthorityRef) -> Self {
        Self::Authority(value)
    }
}

impl From<BooleanOp> for ACEOperand {
    fn from(value: BooleanOp) -> Self {
        Self::BooleanOp(value)
    }
}
impl From<&AuthorityRef> for ACEOperand {
    fn from(value: &AuthorityRef) -> Self {
        Self::Authority(*value)
    }
}

impl From<&BooleanOp> for ACEOperand {
    fn from(value: &BooleanOp) -> Self {
        Self::BooleanOp(*value)
    }
}
