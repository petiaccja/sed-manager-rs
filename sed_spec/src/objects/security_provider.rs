use smallvec::SmallVec;

use crate::objects::{AuthorityRef, SpRef};
use crate::preconfig::core::shared::table_id;
use crate::types::{Date, LifeCycleState};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[sed_spec_macros::object(table=table_id::SP)]
pub struct SecurityProvider {
    pub uid: SpRef,
    pub name: String,
    pub org: AuthorityRef,
    pub effective_auth: SmallVec<[u8; 32]>,
    pub date_of_issue: Date,
    pub bytes: u64,
    pub life_cycle_state: LifeCycleState,
    pub frozen: bool,
}

#[cfg(test)]
mod tests {
    use sed_packet::{Field, FieldRef, Object};

    use super::*;

    const ADMIN_SP: SpRef = SpRef::new_unchecked(0x0000_0205_0000_0001);

    fn get<O: Object + Field<FIELD>, const TABLE: u64, const FIELD: u16>(
        _field: FieldRef<O, TABLE, FIELD>,
    ) -> <O as Field<FIELD>>::Type {
        todo!()
    }

    fn set<O: Object + Field<FIELD>, const TABLE: u64, const FIELD: u16>(
        _field: FieldRef<O, TABLE, FIELD>,
        _value: <O as Field<FIELD>>::Type,
    ) {
        todo!()
    }

    fn foo() -> String {
        let name = get(SecurityProvider::name(ADMIN_SP));
        set(SecurityProvider::name(ADMIN_SP), "asd".into());
        name
    }

    #[test]
    fn test() {
        let value = foo();
        println!("{value}");
    }
}
