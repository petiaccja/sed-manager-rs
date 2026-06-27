mod device;
mod discovery;
mod primitives;
mod properties;
mod uid;
mod user;

pub use properties::CombinedProperties;

pub trait DisplayUi {
    type Ui;

    fn display_ui(&self) -> Self::Ui;
}

impl<T: DisplayUi> DisplayUi for &T {
    type Ui = <T as DisplayUi>::Ui;

    fn display_ui(&self) -> Self::Ui {
        (*self).display_ui()
    }
}
