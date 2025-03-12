use as_array::AsArray;

use crate::spec::basic_types::{List, Set};
use crate::spec::column_types::{ACEOperand, ACERef, AuthorityRef, BooleanOp, Name};

use super::cell::Cell;

#[derive(AsArray)]
#[as_array_traits(Cell)]
pub struct ACE {
    pub uid: ACERef,
    pub name: Name,
    pub common_name: Name,
    pub boolean_expr: List<ACEOperand>,
    pub columns: Set<u16>,
}

impl ACE {
    pub const UID: u16 = 0;
    pub const NAME: u16 = 1;
    pub const COMMON_NAME: u16 = 2;
    pub const BOOLEAN_EXPR: u16 = 3;
    pub const COLUMNS: u16 = 4;
}

pub trait ACEExpr {
    fn eval(&self, authenticated: &[AuthorityRef]) -> Option<bool>;
    fn allow_authority(&self, authority: AuthorityRef) -> Option<Vec<ACEOperand>>;
    fn deny_authority(&self, authority: AuthorityRef) -> Option<Vec<ACEOperand>>;
    fn normalize(&self) -> Option<Vec<ACEOperand>>;
}

impl Default for ACE {
    fn default() -> Self {
        Self {
            uid: ACERef::null(),
            name: Name::default(),
            common_name: Name::default(),
            boolean_expr: List::new(),
            columns: Set::new(),
        }
    }
}

impl<Sequence> ACEExpr for Sequence
where
    for<'seq> &'seq Sequence: IntoIterator<Item = &'seq ACEOperand>,
{
    fn eval(&self, authenticated: &[AuthorityRef]) -> Option<bool> {
        let mut stack = Vec::<bool>::new();
        for item in self.into_iter() {
            match item {
                ACEOperand::Authority(authority) => {
                    stack.push(authenticated.contains(&authority));
                }
                ACEOperand::BooleanOp(BooleanOp::And) => {
                    let rhs = stack.pop()?;
                    let lhs = stack.pop()?;
                    stack.push(lhs && rhs);
                }
                ACEOperand::BooleanOp(BooleanOp::Or) => {
                    let rhs = stack.pop()?;
                    let lhs = stack.pop()?;
                    stack.push(lhs || rhs);
                }
                ACEOperand::BooleanOp(BooleanOp::Not) => {
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

    fn allow_authority(&self, authority: AuthorityRef) -> Option<Vec<ACEOperand>> {
        let already_allowed = self.eval(&[authority])?;
        if already_allowed {
            Some(self.into_iter().cloned().collect())
        } else {
            let mut new_expr: Vec<ACEOperand> = self.into_iter().cloned().collect();
            new_expr.push(ACEOperand::Authority(authority));
            if new_expr.len() != 1 {
                new_expr.push(ACEOperand::BooleanOp(BooleanOp::Or));
            }
            Some(new_expr)
        }
    }

    fn deny_authority(&self, authority: AuthorityRef) -> Option<Vec<ACEOperand>> {
        // Input must already be normalized.
        let normalized = self.normalize()?;
        let mut stack = Vec::<Vec<ACEOperand>>::new();
        let pattern = [ACEOperand::from(authority)];
        for item in normalized {
            match item {
                ACEOperand::Authority(authority) => {
                    stack.push(vec![authority.into()]);
                }
                ACEOperand::BooleanOp(BooleanOp::And) => {
                    let rhs = stack.pop()?;
                    let lhs = stack.pop()?;
                    let op = [BooleanOp::And.into()];
                    let evaled = lhs.into_iter().chain(rhs.into_iter()).chain(op.into_iter());
                    stack.push(evaled.collect());
                }
                ACEOperand::BooleanOp(BooleanOp::Or) => {
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
                ACEOperand::BooleanOp(BooleanOp::Not) => {
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
        if new_expr.as_slice() == pattern {
            Some(vec![])
        } else {
            Some(new_expr)
        }
    }

    fn normalize(&self) -> Option<Vec<ACEOperand>> {
        // Applies the following normalization patterns:
        // - X NOT NOT => X
        // - X X OR => X
        // - X AND X => X
        let mut stack = Vec::<Vec<ACEOperand>>::new();
        for item in self.into_iter() {
            match item {
                ACEOperand::Authority(authority) => {
                    stack.push(vec![authority.into()]);
                }
                ACEOperand::BooleanOp(BooleanOp::And) => {
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
                ACEOperand::BooleanOp(BooleanOp::Or) => {
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
                ACEOperand::BooleanOp(BooleanOp::Not) => {
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

macro_rules! ace_operand {
    (||) => {
        ::sed_manager::spec::column_types::ACEOperand::BooleanOp(::sed_manager::spec::column_types::BooleanOp::Or)
    };
    (&&) => {
        ::sed_manager::spec::column_types::ACEOperand::BooleanOp(::sed_manager::spec::column_types::BooleanOp::And)
    };
    (!) => {
        ::sed_manager::spec::column_types::ACEOperand::BooleanOp(::sed_manager::spec::column_types::BooleanOp::Not)
    };
    ($authority:expr) => {
        ::sed_manager::spec::column_types::ACEOperand::Authority($authority)
    };
}

macro_rules! ace_expr {
    ($($operand:tt)*) => {
        ::sed_manager::spec::basic_types::List::<::sed_manager::spec::column_types::ACEOperand>(vec![$(::sed_manager::spec::objects::ace::ace_operand!($operand)),*])
    };
}

pub(crate) use ace_expr;
pub(crate) use ace_operand;

#[cfg(test)]
mod tests {
    use super::*;

    use crate::spec::opal::locking::authority;

    const ALICE: AuthorityRef = authority::USER.nth(1).unwrap();
    const BOB: AuthorityRef = authority::USER.nth(2).unwrap();
    const CHARLIE: AuthorityRef = authority::USER.nth(3).unwrap();
    const DAVE: AuthorityRef = authority::USER.nth(4).unwrap();

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
