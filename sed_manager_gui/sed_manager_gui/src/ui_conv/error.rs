//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_packet::TableRef;

#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    #[error("The UID belongs to a different table (expected: `{expected}`, found `{actual}`)")]
    IncorrectUidTable { expected: TableRef, actual: TableRef },
}
