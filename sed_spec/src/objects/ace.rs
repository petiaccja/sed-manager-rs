use std::collections::HashSet;

use sed_packet::ObjectUid;
use sed_spec_macros::{DetokenizeStruct, FieldList, Object, TokenizeStruct};

use crate::objects::{AceRef, AuthorityRef};
use crate::preconfig::core::shared::table_id;
use crate::types::{AceOperand, BooleanOp};

#[macro_export]
macro_rules! ace_operand {
    (||) => {
        ::sed_spec::types::AceOperand::BooleanOp(::sed_spec::types::BooleanOp::Or)
    };
    (&&) => {
        ::sed_spec::types::AceOperand::BooleanOp(::sed_spec::types::BooleanOp::And)
    };
    (!) => {
        ::sed_spec::types::AceOperand::BooleanOp(::sed_spec::types::BooleanOp::Not)
    };
    ($authority:expr) => {
        ::sed_spec::types::AceOperand::Authority($authority)
    };
}

#[macro_export]
macro_rules! ace_expr {
    ($($operand:tt)*) => {
        Vec::<$crate::types::AceOperand>::from(vec![$($crate::objects::ace_operand!($operand)),*])
    };
}

pub use ace_expr;
pub use ace_operand;

#[derive(Debug, Clone, Default, PartialEq, Eq, Object, TokenizeStruct, DetokenizeStruct, FieldList)]
#[object(table = table_id::ACE)]
pub struct Ace {
    pub uid: Option<AceRef>,
    pub name: Option<String>,
    pub common_name: Option<String>,
    pub boolean_expr: Option<Vec<AceOperand>>,
    pub columns: Option<HashSet<u16>>,
}

impl ObjectUid for Ace {
    fn uid(&self) -> Option<Self::Ref> {
        self.uid
    }
}

pub trait AceExpr {
    fn eval(&self, authenticated: &[AuthorityRef]) -> Option<bool>;
    fn allow_authority(&self, authority: AuthorityRef) -> Option<Vec<AceOperand>>;
    fn deny_authority(&self, authority: AuthorityRef) -> Option<Vec<AceOperand>>;
    fn normalize(&self) -> Option<Vec<AceOperand>>;
}

impl<Sequence> AceExpr for Sequence
where
    for<'seq> &'seq Sequence: IntoIterator<Item = &'seq AceOperand>,
{
    fn eval(&self, authenticated: &[AuthorityRef]) -> Option<bool> {
        let mut stack = Vec::<bool>::new();
        for item in self.into_iter() {
            match item {
                AceOperand::Authority(authority) => {
                    stack.push(authenticated.contains(&authority));
                }
                AceOperand::BooleanOp(BooleanOp::And) => {
                    let rhs = stack.pop()?;
                    let lhs = stack.pop()?;
                    stack.push(lhs && rhs);
                }
                AceOperand::BooleanOp(BooleanOp::Or) => {
                    let rhs = stack.pop()?;
                    let lhs = stack.pop()?;
                    stack.push(lhs || rhs);
                }
                AceOperand::BooleanOp(BooleanOp::Not) => {
                    let arg = stack.pop()?;
                    stack.push(!arg);
                }
            }
        }
        if stack.len() >= 2 {
            None
        } else {
            stack.first().cloned().or(Some(false))
        }
    }

    fn allow_authority(&self, authority: AuthorityRef) -> Option<Vec<AceOperand>> {
        let already_allowed = self.eval(&[authority])?;
        if already_allowed {
            Some(self.into_iter().cloned().collect())
        } else {
            let mut new_expr: Vec<AceOperand> = self.into_iter().cloned().collect();
            new_expr.push(AceOperand::Authority(authority));
            if new_expr.len() != 1 {
                new_expr.push(AceOperand::BooleanOp(BooleanOp::Or));
            }
            Some(new_expr)
        }
    }

    fn deny_authority(&self, authority: AuthorityRef) -> Option<Vec<AceOperand>> {
        // Input must already be normalized.
        let normalized = self.normalize()?;
        let mut stack = Vec::<Vec<AceOperand>>::new();
        let pattern = [AceOperand::from(authority)];
        for item in normalized {
            match item {
                AceOperand::Authority(authority) => {
                    stack.push(vec![authority.into()]);
                }
                AceOperand::BooleanOp(BooleanOp::And) => {
                    let rhs = stack.pop()?;
                    let lhs = stack.pop()?;
                    let op = [BooleanOp::And.into()];
                    let evaled = lhs.into_iter().chain(rhs.into_iter()).chain(op.into_iter());
                    stack.push(evaled.collect());
                }
                AceOperand::BooleanOp(BooleanOp::Or) => {
                    let rhs = stack.pop()?;
                    let lhs = stack.pop()?;
                    let op = [BooleanOp::Or.into()];
                    if lhs.as_slice() == pattern {
                        stack.push(rhs);
                    } else if rhs.as_slice() == pattern {
                        stack.push(lhs);
                    } else {
                        let evaled = lhs.into_iter().chain(rhs.into_iter()).chain(op.into_iter());
                        stack.push(evaled.collect());
                    }
                }
                AceOperand::BooleanOp(BooleanOp::Not) => {
                    let arg = stack.pop()?;
                    let op = [BooleanOp::Not.into()];
                    let evaled = arg.into_iter().chain(op.into_iter());
                    stack.push(evaled.collect());
                }
            }
        }
        if stack.len() > 1 {
            return None;
        }
        let new_expr = stack.pop().unwrap_or(vec![]);
        if new_expr.as_slice() == pattern { Some(vec![]) } else { Some(new_expr) }
    }

    fn normalize(&self) -> Option<Vec<AceOperand>> {
        // Applies the following normalization patterns:
        // - X NOT NOT => X
        // - X X OR => X
        // - X AND X => X
        let mut stack = Vec::<Vec<AceOperand>>::new();
        for item in self.into_iter() {
            match item {
                AceOperand::Authority(authority) => {
                    stack.push(vec![authority.into()]);
                }
                AceOperand::BooleanOp(BooleanOp::And) => {
                    let rhs = stack.pop()?;
                    let lhs = stack.pop()?;
                    let op = [BooleanOp::And.into()];
                    if lhs.as_slice() == rhs.as_slice() {
                        stack.push(rhs);
                    } else {
                        let evaled = lhs.into_iter().chain(rhs.into_iter()).chain(op.into_iter());
                        stack.push(evaled.collect());
                    }
                }
                AceOperand::BooleanOp(BooleanOp::Or) => {
                    let rhs = stack.pop()?;
                    let lhs = stack.pop()?;
                    let op = [BooleanOp::Or.into()];
                    if lhs.as_slice() == rhs.as_slice() {
                        stack.push(rhs);
                    } else {
                        let evaled = lhs.into_iter().chain(rhs.into_iter()).chain(op.into_iter());
                        stack.push(evaled.collect());
                    }
                }
                AceOperand::BooleanOp(BooleanOp::Not) => {
                    let mut arg = stack.pop()?;
                    let op = [BooleanOp::Not.into()];
                    if arg.last() == Some(&BooleanOp::Not.into()) {
                        arg.pop();
                        stack.push(arg);
                    } else {
                        let evaled = arg.into_iter().chain(op.into_iter());
                        stack.push(evaled.collect());
                    }
                }
            }
        }
        if stack.len() > 1 {
            return None;
        }
        Some(stack.pop().unwrap_or(vec![]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::preconfig::opal_2::locking::authority;

    const ALICE: AuthorityRef = authority::USER.get_unwrap(1);
    const BOB: AuthorityRef = authority::USER.get_unwrap(2);
    const CHARLIE: AuthorityRef = authority::USER.get_unwrap(3);
    const DAVE: AuthorityRef = authority::USER.get_unwrap(4);

    #[test]
    fn eval_ace_expr_empty() {
        let ace_expr = vec![];
        assert_eq!(ace_expr.eval(&[ALICE]), Some(false));
        assert_eq!(ace_expr.eval(&[]), Some(false));
    }

    #[test]
    fn eval_ace_expr_too_many_ops() {
        let ace_expr = ace_expr!(ALICE CHARLIE || ||);
        assert_eq!(ace_expr.eval(&[ALICE]), None);
    }

    #[test]
    fn eval_ace_expr_too_few_ops() {
        let ace_expr = ace_expr!(ALICE BOB CHARLIE ||);
        assert_eq!(ace_expr.eval(&[ALICE]), None);
    }

    #[test]
    fn eval_ace_expr_or() {
        let ace_expr = ace_expr!(ALICE CHARLIE BOB || ||);
        assert_eq!(ace_expr.eval(&[ALICE]), Some(true));
        assert_eq!(ace_expr.eval(&[BOB]), Some(true));
        assert_eq!(ace_expr.eval(&[CHARLIE]), Some(true));
        assert_eq!(ace_expr.eval(&[DAVE]), Some(false));
    }

    #[test]
    fn eval_ace_expr_and() {
        let ace_expr = ace_expr!(ALICE CHARLIE BOB && &&);
        assert_eq!(ace_expr.eval(&[ALICE]), Some(false));
        assert_eq!(ace_expr.eval(&[BOB]), Some(false));
        assert_eq!(ace_expr.eval(&[CHARLIE]), Some(false));
        assert_eq!(ace_expr.eval(&[ALICE, BOB, CHARLIE]), Some(true));
    }

    #[test]
    fn eval_ace_expr_not() {
        let ace_expr = ace_expr!(ALICE !);
        assert_eq!(ace_expr.eval(&[ALICE]), Some(false));
        assert_eq!(ace_expr.eval(&[BOB]), Some(true));
        assert_eq!(ace_expr.eval(&[]), Some(true));
    }

    #[test]
    fn allow_authority_allowed() {
        let ace_expr = ace_expr!(ALICE BOB ||);
        let allowed = ace_expr.allow_authority(BOB);
        assert_eq!(ace_expr.as_slice(), allowed.unwrap().as_slice());
    }

    #[test]
    fn allow_authority_missing() {
        let ace_expr = ace_expr!(ALICE BOB ||);
        let allowed = ace_expr.allow_authority(CHARLIE);
        let expected = ace_expr!(ALICE BOB || CHARLIE ||);
        assert_eq!(expected.as_slice(), allowed.unwrap().as_slice());
    }

    #[test]
    fn allow_authority_only() {
        let ace_expr = ace_expr!();
        let allowed = ace_expr.allow_authority(CHARLIE);
        let expected = ace_expr!(CHARLIE);
        assert_eq!(expected.as_slice(), allowed.unwrap().as_slice());
    }

    #[test]
    fn deny_authority_or_empty() {
        let ace_expr = ace_expr!();
        let denied = ace_expr.deny_authority(DAVE);
        let expected = ace_expr!();
        assert_eq!(expected.as_slice(), denied.unwrap().as_slice());
    }

    #[test]
    fn deny_authority_or_too_many_ops() {
        let ace_expr = ace_expr!(DAVE || ||);
        let denied = ace_expr.deny_authority(DAVE);
        assert!(denied.is_none());
    }
    #[test]
    fn deny_authority_or_too_few_ops() {
        let ace_expr = ace_expr!(DAVE DAVE);
        let denied = ace_expr.deny_authority(DAVE);
        assert!(denied.is_none());
    }

    #[test]
    fn deny_authority_or_lhs() {
        let ace_expr = ace_expr!(DAVE ALICE ||);
        let denied = ace_expr.deny_authority(DAVE);
        let expected = ace_expr!(ALICE);
        assert_eq!(expected.as_slice(), denied.unwrap().as_slice());
    }

    #[test]
    fn deny_authority_or_rhs() {
        let ace_expr = ace_expr!(ALICE DAVE ||);
        let denied = ace_expr.deny_authority(DAVE);
        let expected = ace_expr!(ALICE);
        assert_eq!(expected.as_slice(), denied.unwrap().as_slice());
    }

    #[test]
    fn deny_authority_and_lhs() {
        let ace_expr = ace_expr!(DAVE ALICE &&);
        let denied = ace_expr.deny_authority(DAVE);
        assert_eq!(ace_expr.as_slice(), denied.unwrap().as_slice());
    }

    #[test]
    fn deny_authority_and_rhs() {
        let ace_expr = ace_expr!(ALICE DAVE &&);
        let denied = ace_expr.deny_authority(DAVE);
        assert_eq!(ace_expr.as_slice(), denied.unwrap().as_slice());
    }

    #[test]
    fn deny_authority_or_repeated() {
        let ace_expr = ace_expr!(DAVE DAVE ||);
        let denied = ace_expr.deny_authority(DAVE);
        let expected = ace_expr!();
        assert_eq!(expected.as_slice(), denied.unwrap().as_slice());
    }

    #[test]
    fn deny_authority_and_repeated() {
        let ace_expr = ace_expr!(DAVE DAVE &&);
        let denied = ace_expr.deny_authority(DAVE);
        let expected = ace_expr!();
        assert_eq!(expected.as_slice(), denied.unwrap().as_slice());
    }

    #[test]
    fn deny_authority_not_repeated() {
        let ace_expr = ace_expr!(DAVE ! !);
        let denied = ace_expr.deny_authority(DAVE);
        let expected = ace_expr!();
        assert_eq!(expected.as_slice(), denied.unwrap().as_slice());
    }
}
