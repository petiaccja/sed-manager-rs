mod device;
mod discovery;
mod primitives;
mod properties;
mod uid;
mod user;

pub use properties::CombinedProperties;
use sed_packet::discovery::FeatureDescriptor;
use sed_spec::objects::SecurityProviderRef;

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

pub trait DisplayUiName {
    type Ui;

    fn display_ui_name(&self, features: &[FeatureDescriptor], sp: Option<SecurityProviderRef>) -> Self::Ui;
}

impl<T: DisplayUiName> DisplayUiName for &T {
    type Ui = <T as DisplayUiName>::Ui;

    fn display_ui_name(&self, features: &[FeatureDescriptor], sp: Option<SecurityProviderRef>) -> Self::Ui {
        (*self).display_ui_name(features, sp)
    }
}
