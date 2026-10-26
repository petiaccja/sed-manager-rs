//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::num::NonZero;
use std::time::Duration;

use sed_spec::methods::{Limit, OptionalLimit};
use slint::{SharedString, ToSharedString};

use crate::ui_conv::{FromBackend, IntoUi};

impl FromBackend<&Duration> for SharedString {
    fn from_backend(value: &Duration) -> Self {
        if value < &Duration::from_micros(1) {
            format!("{} ns", value.as_nanos()).into()
        } else if value < &Duration::from_millis(1) {
            format!("{} μs", value.as_nanos() as f64 / 1e3).into()
        } else if value < &Duration::from_secs(1) {
            format!("{} ms", value.as_millis() as f64 / 1e3).into()
        } else {
            let seconds = (value.as_millis() % 60_000) as f64 / 1e3;
            let minutes = value.as_secs() / 60 % 60;
            let hours = value.as_secs() / 3600;
            if hours > 0 {
                format!("{}h {}m {}s", hours, minutes, seconds).into()
            } else if minutes > 0 {
                format!("{}m {}s", minutes, seconds).into()
            } else {
                format!("{} s", seconds).into()
            }
        }
    }
}

impl<'a, T> FromBackend<&'a Option<T>> for SharedString
where
    SharedString: FromBackend<&'a T>,
{
    fn from_backend(value: &'a Option<T>) -> Self {
        value.as_ref().map(|value| value.into_ui()).unwrap_or("-".into())
    }
}

impl<'a, T> FromBackend<&'a Limit<T>> for SharedString
where
    SharedString: FromBackend<&'a T>,
{
    fn from_backend(value: &'a Limit<T>) -> Self {
        match value {
            Limit::Unlimited => "unlimited".into(),
            Limit::Limited(value) => value.into_ui(),
        }
    }
}

impl<'a, T> FromBackend<&'a OptionalLimit<T>> for SharedString
where
    SharedString: FromBackend<&'a T>,
{
    fn from_backend(value: &'a OptionalLimit<T>) -> Self {
        match value {
            OptionalLimit::Unsupported => "not supported".into(),
            OptionalLimit::Limited(value) => value.into_ui(),
        }
    }
}

impl FromBackend<&NonZero<usize>> for SharedString {
    fn from_backend(value: &NonZero<usize>) -> Self {
        value.get().to_string().into()
    }
}

impl FromBackend<&usize> for SharedString {
    fn from_backend(value: &usize) -> Self {
        value.to_string().into()
    }
}

impl FromBackend<&bool> for SharedString {
    fn from_backend(value: &bool) -> Self {
        match value {
            true => "yes",
            false => "no",
        }
        .to_shared_string()
    }
}
