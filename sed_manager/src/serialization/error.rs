#[derive(Debug)]
pub enum SerializeError {
    SeekFailed,
    EndOfStream,
    InvalidRepresentation,
    Other(Box<dyn std::error::Error>),
}

impl std::error::Error for SerializeError {}

impl std::fmt::Display for SerializeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerializeError::Other(err) => f.write_fmt(format_args!("other: `{}`", err.as_ref())),
            SerializeError::SeekFailed => f.write_fmt(format_args!("seek failed")),
            SerializeError::EndOfStream => f.write_fmt(format_args!("end of stream")),
            SerializeError::InvalidRepresentation => {
                f.write_fmt(format_args!("invalid serialized representation of object"))
            }
        }
    }
}
