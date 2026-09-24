//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

mod authority;
mod device;
mod discovery;
mod locking_range;
mod mbr;
mod primitives;
mod properties;
mod uid;

pub use mbr::MbrDesc;
pub use properties::CombinedProperties;
use sed_packet::discovery::FeatureDescriptor;
use sed_spec::objects::SecurityProviderRef;

pub trait IntoUi {
    type Ui;

    #[expect(clippy::wrong_self_convention, reason = "fix later")]
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

    #[expect(clippy::wrong_self_convention, reason = "fix later")]
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
