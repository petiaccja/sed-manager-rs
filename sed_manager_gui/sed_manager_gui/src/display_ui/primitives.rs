use std::{num::NonZero, time::Duration};

use sed_spec::methods::{Limit, OptionalLimit};
use slint::{SharedString, ToSharedString};

use crate::display_ui::DisplayUi;

impl DisplayUi for Duration {
    type Ui = SharedString;

    fn display_ui(&self) -> Self::Ui {
        if self < &Duration::from_micros(1) {
            format!("{} ns", self.as_nanos()).into()
        } else if self < &Duration::from_millis(1) {
            format!("{} μs", self.as_nanos() as f64 / 1e3).into()
        } else if self < &Duration::from_secs(1) {
            format!("{} ms", self.as_millis() as f64 / 1e3).into()
        } else {
            let seconds = (self.as_millis() % 60_000) as f64 / 1e3;
            let minutes = self.as_secs() / 60 % 60;
            let hours = self.as_secs() / 3600;
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

impl<T> DisplayUi for Option<T>
where
    T: DisplayUi<Ui = SharedString>,
{
    type Ui = SharedString;

    fn display_ui(&self) -> Self::Ui {
        self.as_ref().map(|value| value.display_ui()).unwrap_or("-".into())
    }
}

impl<T> DisplayUi for Limit<T>
where
    T: DisplayUi<Ui = SharedString>,
{
    type Ui = SharedString;

    fn display_ui(&self) -> Self::Ui {
        match self {
            Limit::Unlimited => "unlimited".into(),
            Limit::Limited(value) => value.display_ui(),
        }
    }
}

impl<T> DisplayUi for OptionalLimit<T>
where
    T: DisplayUi<Ui = SharedString>,
{
    type Ui = SharedString;

    fn display_ui(&self) -> Self::Ui {
        match self {
            OptionalLimit::Unsupported => "not supported".into(),
            OptionalLimit::Limited(value) => value.display_ui(),
        }
    }
}

impl DisplayUi for NonZero<usize> {
    type Ui = SharedString;

    fn display_ui(&self) -> Self::Ui {
        self.get().display_ui()
    }
}

impl DisplayUi for usize {
    type Ui = SharedString;

    fn display_ui(&self) -> Self::Ui {
        self.to_string().into()
    }
}

impl DisplayUi for bool {
    type Ui = SharedString;

    fn display_ui(&self) -> Self::Ui {
        match self {
            true => "yes",
            false => "no",
        }
        .to_shared_string()
    }
}
