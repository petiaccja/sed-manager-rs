mod device;
mod discovery;

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
