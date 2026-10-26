//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::io;
use std::path::PathBuf;

use sed_manager::Error as SedError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("The command was skipped.")]
    CommandSkipped,
    #[error(transparent)]
    Sed(#[from] SedError),
    #[error(transparent)]
    File(#[from] FileError),
    #[error("The command was cancelled. Results may be partially applied.")]
    CommandCancelled,
}

#[derive(Debug, thiserror::Error)]
pub enum FileError {
    #[error("{file}: {cause}")]
    Io {
        file: PathBuf,
        #[source]
        cause: io::Error,
    },
    #[error("The file can be at maximum {maximum_size} bytes, but it is {actual_size} bytes")]
    FileTooBig { maximum_size: u64, actual_size: u64 },
}
