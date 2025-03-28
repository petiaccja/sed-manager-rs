//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Field { name: String, error: Box<Error> },
    InvalidData,
    EndOfStream,
    Unspecified,
}

impl Error {
    pub fn field(name: String, error: Error) -> Self {
        Self::Field { name: name, error: Box::new(error) }
    }
}

impl core::error::Error for Error {}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut path = Vec::<&str>::new();
        let mut item = self;
        while let Self::Field { name, error } = item {
            path.push(name);
            item = error.as_ref();
        }
        f.write_fmt(format_args!("serialization failed:"))?;
        match item {
            Error::Unspecified => f.write_fmt(format_args!("unknown serialization error"))?,
            Error::EndOfStream => f.write_fmt(format_args!("end of stream"))?,
            Error::InvalidData => f.write_fmt(format_args!("invalid data"))?,
            Error::Field { name: _, error: _ } => unreachable!(),
        };
        if !path.is_empty() {
            f.write_fmt(format_args!(" for {}", path.join("::")))?;
        }
        Ok(())
    }
}

pub fn annotate_field<T, E: Into<Error>>(result: Result<T, E>, field: String) -> Result<T, Error> {
    match result {
        Ok(value) => Ok(value),
        Err(err) => Err(Error::field(field, err.into())),
    }
}
