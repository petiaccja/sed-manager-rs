//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_manager_gui_slint as ui;
use sed_packet::{ObjectRef, TableRef, Uid, discovery::FeatureDescriptor};
use sed_spec::{objects::SecurityProviderRef, path::Path, preconfig::GLOBAL_LOOKUP};
use slint::SharedString;

use crate::ui_conv::{ConversionError, FromBackend, FromBackendName, FromUi, TryFromUi};

impl FromBackend<&Uid> for ui::Uid {
    fn from_backend(value: &Uid) -> Self {
        ui::Uid { value: value.to_u64().cast_signed() }
    }
}

impl<const TABLE: u64> FromBackend<&ObjectRef<TABLE>> for ui::Uid {
    fn from_backend(value: &ObjectRef<TABLE>) -> Self {
        Self::from_backend(&value.to_uid())
    }
}

impl FromUi<&ui::Uid> for Uid {
    fn from_ui(value: &ui::Uid) -> Self {
        Self::new(value.value.cast_unsigned())
    }
}

impl FromBackendName<&Uid> for SharedString {
    fn from_backend_name(value: &Uid, features: &[FeatureDescriptor], sp: Option<SecurityProviderRef>) -> Self {
        GLOBAL_LOOKUP
            .by_uid(features, *value, sp)
            .map(|name| Path::new(&name).object().to_owned())
            .unwrap_or_else(|| value.to_string())
            .into()
    }
}

impl<const TABLE: u64> FromBackendName<&ObjectRef<TABLE>> for SharedString {
    fn from_backend_name(
        value: &ObjectRef<TABLE>,
        features: &[FeatureDescriptor],
        sp: Option<SecurityProviderRef>,
    ) -> Self {
        Self::from_backend_name(&value.to_uid(), features, sp)
    }
}

impl<'a, const TABLE: u64> TryFromUi<&'a ui::Uid> for ObjectRef<TABLE> {
    type Error = ConversionError;

    fn try_from_ui(value: &'a ui::Uid) -> Result<Self, Self::Error> {
        let uid = Uid::from_ui(value);
        let expected = TableRef::try_new(TABLE).expect("should always be a table reference");
        let actual = uid.containing_table().and_then(|uid| TableRef::try_from(uid).ok()).unwrap_or(TableRef::null());
        Self::try_from(uid).map_err(|_| ConversionError::IncorrectUidTable { expected, actual })
    }
}
