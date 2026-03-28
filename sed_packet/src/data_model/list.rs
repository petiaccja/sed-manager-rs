use crate::token::{Detokenize, Detokenizer, Tokenize, Tokenizer};

impl<Item> Tokenize for Vec<Item>
where
    Item: Tokenize,
{
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        tokenizer.tokenize_list(|tokenizer| {
            for item in self {
                item.tokenize(tokenizer)?;
            }
            Ok(())
        })
    }
}

impl<Item> Detokenize for Vec<Item>
where
    Item: Detokenize,
{
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        let mut items = Vec::new();
        detokenizer
            .detokenize_list(|detokenizer| {
                items.push(Item::detokenize(detokenizer)?);
                Ok(())
            })
            .map(|_| items)
    }
}
