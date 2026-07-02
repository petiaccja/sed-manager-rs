mod authority;
mod device;
mod discovery;
mod primitives;
mod properties;
mod uid;

pub use properties::CombinedProperties;
use sed_packet::discovery::FeatureDescriptor;
use sed_spec::objects::SecurityProviderRef;

pub trait IntoUi {
    type Ui;

    fn into_ui(&self) -> Self::Ui;
}

impl<T: IntoUi> IntoUi for &T {
    type Ui = <T as IntoUi>::Ui;

    fn into_ui(&self) -> Self::Ui {
        (*self).into_ui()
    }
}

pub trait IntoUiName {
    type Ui;

    fn into_ui_name(&self, features: &[FeatureDescriptor], sp: Option<SecurityProviderRef>) -> Self::Ui;
}

impl<T: IntoUiName> IntoUiName for &T {
    type Ui = <T as IntoUiName>::Ui;

    fn into_ui_name(&self, features: &[FeatureDescriptor], sp: Option<SecurityProviderRef>) -> Self::Ui {
        (*self).into_ui_name(features, sp)
    }
}

pub trait TryFromUi<T> {
    type Error;
    fn try_from_ui(value: T) -> Result<Self, Self::Error>
    where
        Self: Sized;
}
