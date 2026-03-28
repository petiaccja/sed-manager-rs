use crate::token::{Detokenize, Detokenizer, Tokenize, Tokenizer};

pub struct Named<Name, Value> {
    pub name: Name,
    pub value: Value,
}

impl<Name, Value> Tokenize for Named<Name, Value>
where
    Name: Tokenize,
    Value: Tokenize,
{
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        tokenizer.tokenize_named(&self.name, &self.value)
    }
}

impl<Name, Value> Detokenize for Named<Name, Value>
where
    Name: Detokenize,
    Value: Detokenize,
{
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        detokenizer
            .detokenize_named(
                |detokenizer| Name::detokenize(detokenizer),
                |detokenizer, _| Value::detokenize(detokenizer),
            )
            .map(|(name, value)| Named { name, value })
    }
}
