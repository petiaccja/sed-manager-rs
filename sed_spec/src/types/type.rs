use crate::objects::TypeRef;

pub trait Type {
    const UID: TypeRef;
}
