//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

pub use skip_test_macros::may_skip;

pub struct Skip;

pub struct Skipped;

impl From<Skipped> for () {
    fn from(_: Skipped) -> Self {
        ()
    }
}

impl<E> From<Skipped> for Result<(), E> {
    fn from(_: Skipped) -> Self {
        Ok(())
    }
}

pub trait TryUnwrap {
    type Value;
    fn try_unwrap(self) -> Option<Self::Value>;
}

impl<T, E> TryUnwrap for Result<T, E> {
    type Value = T;
    fn try_unwrap(self) -> Option<Self::Value> {
        match self {
            Ok(value) => Some(value),
            Err(_) => None,
        }
    }
}

impl<T> TryUnwrap for Option<T> {
    type Value = T;
    fn try_unwrap(self) -> Option<Self::Value> {
        self
    }
}

#[macro_export]
macro_rules! skip {
    () => {
        std::panic::panic_any(::skip_test::Skip {});
    };
}

#[macro_export]
macro_rules! skip_or_unwrap {
    ($result:expr) => {{
        use ::skip_test::TryUnwrap;
        match $result.try_unwrap() {
            None => std::panic::panic_any(::skip_test::Skip {}),
            Some(value) => value,
        }
    }};
}
