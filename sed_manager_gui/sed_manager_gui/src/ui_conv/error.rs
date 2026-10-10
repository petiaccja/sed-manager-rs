use sed_packet::TableRef;

#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    #[error("The UID belongs to a different table (expected: `{expected}`, found `{actual}`)")]
    IncorrectUidTable { expected: TableRef, actual: TableRef },
}
