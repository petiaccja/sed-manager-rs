use std::collections::{BTreeSet, HashSet};

use crate::token::{Detokenize, Detokenizer, Tokenize, Tokenizer};

impl<Item> Tokenize for HashSet<Item>
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

impl<Item> Detokenize for HashSet<Item>
where
    Item: Detokenize + Eq + core::hash::Hash,
{
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        let mut items = HashSet::new();
        detokenizer
            .detokenize_list(|detokenizer| {
                items.insert(Item::detokenize(detokenizer)?);
                Ok(())
            })
            .map(|_| items)
    }
}

impl<Item> Tokenize for BTreeSet<Item>
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

impl<Item> Detokenize for BTreeSet<Item>
where
    Item: Detokenize + Eq + Ord,
{
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        let mut items = BTreeSet::new();
        detokenizer
            .detokenize_list(|detokenizer| {
                items.insert(Item::detokenize(detokenizer)?);
                Ok(())
            })
            .map(|_| items)
    }
}
