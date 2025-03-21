//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

/// Run block of code and close session asynchronously afterwards.
///
/// While the session would be closed by [`Drop`] without blocking, it might
/// take a while until the protocol thread actually closes the session.
/// This can lead to weird issues like SPBusy when opening the next session.
/// This macro ensures the session really is closed before returning.
macro_rules! with_session {
    ($id:ident = $session:expr => $block:expr) => {{
        let $id = $session;
        let result = async { $block }.await;
        let _ = $id.end_session().await;
        result
    }};
    ($id:ident => $block:expr) => {{
        let result = async { $block }.await;
        let _ = $id.end_session().await;
        result
    }};
}

pub(crate) use with_session;
