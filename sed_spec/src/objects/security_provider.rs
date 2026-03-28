use sed_packet::token::MessageError;
use sed_packet::{ObjectRef, TableRef};

const SP_TABLE: TableRef = TableRef::new_unchecked(0x0000_0205_0000_0000);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[sed_spec_macros::object(table=SP_TABLE)]
pub struct SecurityProvider {
    uid: SecurityProviderRef,
    name: String,
}

pub type SecurityProviderRef = ObjectRef<{ SP_TABLE.to_u64() }>;

#[cfg(test)]
mod tests {
    use sed_packet::{Field, FieldRef, Object};

    use super::*;

    const ADMIN_SP: SecurityProviderRef = SecurityProviderRef::new_unchecked(0x0000_0205_0000_0001);

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
