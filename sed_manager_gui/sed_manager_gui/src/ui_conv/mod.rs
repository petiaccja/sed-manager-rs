//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

mod authority;
mod device;
mod discovery;
mod geometry;
mod locking_range;
mod mbr;
mod primitives;
mod properties;
mod uid;

use std::convert::Infallible;

pub use mbr::MbrDesc;
pub use properties::CombinedProperties;
use sed_packet::discovery::FeatureDescriptor;
use sed_spec::objects::SecurityProviderRef;

//------------------------------------------------------------------------------
// Backend -> UI
//------------------------------------------------------------------------------

pub trait FromBackend<Backend>: Sized {
    fn from_backend(value: Backend) -> Self;
}

impl<Ui: FromBackendName<Backend>, Backend> FromBackend<Backend> for Ui {
    fn from_backend(value: Backend) -> Self {
        Self::from_backend_name(value, &[], None)
    }
}

//--------------------------------------
// Into counterparts.
//--------------------------------------

pub trait IntoUi<Ui> {
    fn into_ui(self) -> Ui;
}

impl<Ui: FromBackend<Backend>, Backend> IntoUi<Ui> for Backend {
    fn into_ui(self) -> Ui {
        Ui::from_backend(self)
    }
}

//------------------------------------------------------------------------------
// Backend -> UI (with UID -> Name)
//------------------------------------------------------------------------------

pub trait FromBackendName<Backend>: Sized {
    fn from_backend_name(value: Backend, features: &[FeatureDescriptor], sp: Option<SecurityProviderRef>) -> Self;
}

//--------------------------------------
// Into counterparts.
//--------------------------------------

pub trait IntoUiName<Ui> {
    fn into_ui_name(self, features: &[FeatureDescriptor], sp: Option<SecurityProviderRef>) -> Ui;
}

impl<Ui: FromBackendName<Backend>, Backend> IntoUiName<Ui> for Backend {
    fn into_ui_name(self, features: &[FeatureDescriptor], sp: Option<SecurityProviderRef>) -> Ui {
        Ui::from_backend_name(self, features, sp)
    }
}

//------------------------------------------------------------------------------
// UI -> Backend
//------------------------------------------------------------------------------

pub trait TryFromUi<Ui>: Sized {
    type Error;
    fn try_from_ui(value: Ui) -> Result<Self, Self::Error>;
}

pub trait FromUi<Ui>: Sized {
    fn from_ui(value: Ui) -> Self;
}

impl<Backend: FromUi<Ui>, Ui> TryFromUi<Ui> for Backend {
    type Error = Infallible;

    fn try_from_ui(value: Ui) -> Result<Self, Self::Error> {
        Ok(Self::from_ui(value))
    }
}

//--------------------------------------
// Into counterparts.
//--------------------------------------

pub trait IntoBackend<Backend> {
    fn into_backend(self) -> Backend;
}

impl<Backend: FromUi<Ui>, Ui> IntoBackend<Backend> for Ui {
    fn into_backend(self) -> Backend {
        Backend::from_ui(self)
    }
}

pub trait TryIntoBackend<Backend> {
    type Error;
    fn try_into_backend(self) -> Result<Backend, Self::Error>;
}

impl<Backend: TryFromUi<Ui>, Ui> TryIntoBackend<Backend> for Ui {
    type Error = <Backend as TryFromUi<Ui>>::Error;
    fn try_into_backend(self) -> Result<Backend, Self::Error> {
        Backend::try_from_ui(self)
    }
}
