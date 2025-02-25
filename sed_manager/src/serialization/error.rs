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
    pub fn io(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::InvalidData => Error::InvalidData,
            std::io::ErrorKind::InvalidInput => Error::InvalidData,
            std::io::ErrorKind::UnexpectedEof => Error::EndOfStream,
            _ => Error::Unspecified,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::io(value)
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
