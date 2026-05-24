mod discovery;

pub trait DisplayUi {
    type Ui;

    fn display_ui(&self) -> Self::Ui;
}
