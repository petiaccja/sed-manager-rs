#[derive(Debug)]
pub enum Error {
    Field { name: String, error: Box<Error> },
    IO { error: std::io::Error, stream_pos: Option<u64> },
    Other { error: Box<dyn std::error::Error + Sync + Send>, stream_pos: Option<u64> },
}

impl Error {
    pub fn field(name: String, error: Error) -> Self {
        Self::Field { name: name, error: Box::new(error) }
    }
    pub fn io(error: std::io::Error, stream_pos: Option<u64>) -> Self {
        Self::IO { error: error, stream_pos: stream_pos }
    }
    pub fn other<E: std::error::Error + Sync + Send + 'static>(error: E, stream_pos: Option<u64>) -> Self {
        Self::Other { error: Box::new(error), stream_pos: stream_pos }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::io(value, None)
    }
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut path = Vec::<&str>::new();
        let mut item = self;
        while let Self::Field { name, error } = item {
            path.push(name);
            item = error.as_ref();
        }
        f.write_fmt(format_args!("serialization failed:"))?;
        match item {
            Error::IO { error, stream_pos } => {
                if let Some(value) = stream_pos {
                    f.write_fmt(format_args!(" @{}", value))?;
                };
                f.write_fmt(format_args!(" {}", error))?;
            }
            Error::Other { error, stream_pos } => {
                if let Some(value) = stream_pos {
                    f.write_fmt(format_args!(" @{}", value))?;
                };
                f.write_fmt(format_args!(" {}", *error))?;
            }
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
