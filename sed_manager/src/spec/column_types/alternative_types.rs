use sed_manager_macros::AlternativeType;

use super::{AuthorityRef, BooleanOp, Bytes32, Bytes64};

#[derive(AlternativeType, PartialEq, Eq, Clone, Debug)]
pub enum Key256 {
    Bytes32(Bytes32),
    Bytes64(Bytes64),
}

#[derive(AlternativeType, PartialEq, Eq, Clone, Debug)]
pub enum ACEExpression {
    Authority(AuthorityRef),
    BooleanOp(BooleanOp),
}
