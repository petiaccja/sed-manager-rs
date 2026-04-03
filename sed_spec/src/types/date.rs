use sed_packet::Named;
use sed_packet::token::{Detokenize, Detokenizer, MessageError, Tokenize, Tokenizer};

#[derive(PartialEq, Eq, Clone, Debug, Default)]
pub struct Date {
    year: Option<u16>,
    month: Option<u8>,
    day: Option<u8>,
}

impl Tokenize for Date {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        tokenizer.tokenize_list(|tokenizer| {
            self.year.map(|year| Named { name: 0, value: year }.tokenize(tokenizer)).transpose()?;
            self.month.map(|month| Named { name: 0, value: month }.tokenize(tokenizer)).transpose()?;
            self.day.map(|day| Named { name: 0, value: day }.tokenize(tokenizer)).transpose()?;
            Ok(())
        })
    }
}

impl Detokenize for Date {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        let mut date = Date::default();
        detokenizer.detokenize_list(|de| {
            de.detokenize_named(
                |de| u8::detokenize(de),
                |de, name| match name {
                    0 => {
                        date.year = Some(u16::detokenize(de)?);
                        Ok(())
                    }
                    1 => {
                        date.month = Some(u8::detokenize(de)?);
                        Ok(())
                    }
                    2 => {
                        date.day = Some(u8::detokenize(de)?);
                        Ok(())
                    }
                    _ => Err(D::Error::message("invalid struct member for `Date`")),
                },
            )
            .map(|_| ())
        })?;
        Ok(date)
    }
}
