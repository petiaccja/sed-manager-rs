use as_array::AsArray;

use crate::spec::{
    basic_types::{List, Set},
    column_types::{ACEOperand, ACERef, Name},
};

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
        ::sed_manager::spec::basic_types::List(vec![$(::sed_manager::spec::objects::ace::ace_operand!($operand)),*])
    };
}

pub(crate) use ace_expr;
pub(crate) use ace_operand;
