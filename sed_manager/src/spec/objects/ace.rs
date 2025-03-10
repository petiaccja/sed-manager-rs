use as_array::AsArray;

use crate::spec::{
    basic_types::{List, Set},
    column_types::{ACEExpression, ACERef, Name},
};

use super::cell::Cell;

#[derive(AsArray)]
#[as_array_traits(Cell)]
pub struct ACE {
    pub uid: ACERef,
    pub name: Name,
    pub common_name: Name,
    pub boolean_expr: List<ACEExpression>,
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
