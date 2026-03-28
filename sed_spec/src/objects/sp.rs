use sed_packet::token::{Detokenize, Detokenizer, MessageError, Tokenize, Tokenizer};
use sed_packet::{Field, FieldRef, Named, Object, ObjectRef, TableRef};

const SP_TABLE: TableRef = TableRef::new_unchecked(0x0000_0205_0000_0000);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Sp {
    uid: Option<SpRef>,
    name: Option<String>,
}

impl Sp {
    const fn uid(object: SpRef) -> FieldRef<Self, 0> {
        FieldRef::new(object)
    }

    const fn name(object: SpRef) -> FieldRef<Self, 1> {
        FieldRef::new(object)
    }
}

impl Object for Sp {
    const TABLE: TableRef = SP_TABLE;
}

impl Field<0> for Sp {
    type Type = SpRef;
}

impl Field<1> for Sp {
    type Type = String;
}

impl Tokenize for Sp {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        tokenizer.tokenize_list(|tokenizer| {
            self.uid.as_ref().map(|value| Named { name: 0u16, value }.tokenize(tokenizer)).transpose()?;
            self.name.as_ref().map(|value| Named { name: 1u16, value }.tokenize(tokenizer)).transpose()?;
            Ok(())
        })
    }
}

impl Detokenize for Sp {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        let mut result = Self::default();
        detokenizer.detokenize_list(|de| {
            de.detokenize_named(
                |de| u16::detokenize(de),
                |de, field| match field {
                    0 => {
                        result.uid = Some(SpRef::detokenize(de)?);
                        Ok(())
                    }
                    1 => {
                        result.name = Some(String::detokenize(de)?);
                        Ok(())
                    }
                    _ => Err(D::Error::message("invalid field")),
                },
            )?;
            Ok(())
        })?;
        Ok(result)
    }
}

type SpRef = ObjectRef<Sp>;

#[cfg(test)]
mod tests {
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
