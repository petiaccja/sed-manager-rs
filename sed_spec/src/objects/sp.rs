use sed_packet::token::MessageError;
use sed_packet::{ObjectRef, TableRef};

const SP_TABLE: TableRef = TableRef::new_unchecked(0x0000_0205_0000_0000);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[sed_spec_macros::object(table=SP_TABLE)]
struct Sp {
    uid: SpRef,
    name: String,
}

type SpRef = ObjectRef<Sp>;

#[cfg(test)]
mod tests {
    use sed_packet::{Field, FieldRef, Object};

    use super::*;

    const ADMIN_SP: SpRef = SpRef::new_unchecked(0x0000_0205_0000_0001);

    fn get<O: Object + Field<FIELD>, const FIELD: u16>(_field: FieldRef<O, FIELD>) -> <O as Field<FIELD>>::Type {
        todo!()
    }

    fn set<O: Object + Field<FIELD>, const FIELD: u16>(_field: FieldRef<O, FIELD>, _value: <O as Field<FIELD>>::Type) {
        todo!()
    }

    fn foo() -> String {
        let name = get(Sp::name(ADMIN_SP));
        set(Sp::name(ADMIN_SP), "asd".into());
        name
    }

    #[test]
    fn test() {
        let value = foo();
        println!("{value}");
    }
}
