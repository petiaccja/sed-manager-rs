use std::io;

use sed_manager::Error as SedError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("The command was skipped.")]
    CommandSkipped,
    #[error(transparent)]
    Sed(#[from] SedError),
    #[error(transparent)]
    File(#[from] FileError),
}

#[derive(Debug, thiserror::Error)]
pub enum FileError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("The file can be at maximum {maximum_size} bytes, but it is {actual_size} bytes")]
    FileTooBig { maximum_size: u64, actual_size: u64 },
}
