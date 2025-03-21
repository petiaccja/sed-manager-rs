//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

#[allow(unused)]
macro_rules! with_copy {
    ($object:ident, $action:expr) => {{
        let $object = $object.clone();
        async move { $action.await }
    }};
    ($object:ident, $alias:ident, $action:expr) => {{
        let $alias = $object.clone();
        async move { $action.await }
    }};
}

#[allow(unused)]
pub(crate) use with_copy;
