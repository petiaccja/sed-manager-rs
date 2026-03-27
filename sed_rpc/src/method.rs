use core::marker::PhantomData;

use sed_packet::token::{Tokenize, Tokenizer};

struct Uid(u64);

impl Tokenize for Uid {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        tokenizer.tokenize_bytes(&self.0.to_be_bytes())
    }
}

struct Name(String);

impl Tokenize for Name {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        tokenizer.tokenize_bytes(self.0.as_bytes())
    }
}

pub trait Column<const INDEX: u16> {
    type Type: Tokenize;
}

pub trait Object {
    const TABLE: TableRef;
}

struct TableRef(Uid);

struct ObjectRef<ObjectTy: Object>(Uid, PhantomData<ObjectTy>);

struct MethodId {
    uid: Uid,
    name: Name,
    common_name: Name,
    template_id: Uid,
}

impl Object for MethodId {
    const TABLE: TableRef = TableRef(Uid(0));
}

impl Column<0> for MethodId {
    type Type = Uid;
}

impl Column<1> for MethodId {
    type Type = Name;
}

impl Column<2> for MethodId {
    type Type = Name;
}

impl Column<3> for MethodId {
    type Type = Uid;
}

fn set<Obj, const COLUMN: u16>(object: ObjectRef<Obj>, value: <Obj as Column<COLUMN>>::Type)
where
    Obj: Object,
    Obj: Column<COLUMN>,
{
    todo!()
}
