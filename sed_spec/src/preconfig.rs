#![allow(unused)]

use crate::lookup::GlobalLookup;

include!(concat!(env!("OUT_DIR"), "/spec.rs"));

/// The purpose of this is only so that it's easy to jump into the generated
/// code using "go to definition".
#[allow(unused)]
const MARKER: () = GENERATED_MARKER;

pub const GLOBAL_LOOKUP: GlobalLookup = GlobalLookup;

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        lookup::{FeatureLookup, ObjectTableLookup, SecurityProviderLookup, TableLookup},
        path::Path,
    };
    use sed_packet::Uid;

    use rstest::rstest;

    #[rstest]
    #[case(&self::core::LOOKUP)]
    #[case(&self::enterprise::LOOKUP)]
    #[case(&self::kpio::LOOKUP)]
    #[case(&self::opal_2::LOOKUP)]
    #[case(&self::opalite::LOOKUP)]
    #[case(&self::pyrite_2::LOOKUP)]
    #[case(&self::ruby::LOOKUP)]
    #[case(&self::data_store::LOOKUP)]
    #[case(&self::psid::LOOKUP)]
    fn sortedness_feature_lookups(#[case] lookup: &FeatureLookup) {
        lookup.sp_lookups.items.is_sorted_by_key(|(key, _)| key);
    }

    #[rstest]
    #[case(&self::core::shared::LOOKUP)]
    #[case(&self::opal_2::admin::LOOKUP)]
    #[case(&self::opal_2::locking::LOOKUP)]
    fn sortedness_security_provider_lookups(#[case] lookup: &SecurityProviderLookup) {
        lookup.table_lookups.items.is_sorted_by_key(|(key, _)| key);
    }

    #[test]
    fn sortedness_object_table_lookups() {
        self::core::shared::table::LOOKUP.by_name.items.is_sorted_by_key(|(key, _)| key);
        self::core::shared::table::LOOKUP.by_uid.items.is_sorted_by_key(|(key, _)| key);
    }

    #[test]
    fn sortedness_meta_table_lookups() {
        self::core::shared::method_id::LOOKUP.by_name.items.is_sorted_by_key(|(key, _)| key);
        self::core::shared::method_id::LOOKUP.by_uid.items.is_sorted_by_key(|(key, _)| key);
        self::core::shared::table_id::LOOKUP.by_name.items.is_sorted_by_key(|(key, _)| key);
        self::core::shared::table_id::LOOKUP.by_uid.items.is_sorted_by_key(|(key, _)| key);
    }

    #[rstest]
    #[case("Anybody", Some(self::core::shared::authority::ANYBODY.to_uid()))]
    #[case("Authority::Anybody", Some(self::core::shared::authority::ANYBODY.to_uid()))]
    #[case("Another::Anybody", None)]
    fn lookup_table(#[case] path: impl AsRef<Path>, #[case] uid: Option<Uid>) {
        let path = path.as_ref();
        assert_eq!(self::core::shared::authority::LOOKUP.by_path(path), uid);
        if let Some(uid) = uid {
            assert_eq!(self::core::shared::authority::LOOKUP.by_uid(uid), Some(path.object().to_owned()));
        }
    }

    #[rstest]
    #[case("Authority::Anybody", Some(self::core::shared::authority::ANYBODY.to_uid()))]
    #[case("Shared::Authority::Anybody", Some(self::core::shared::authority::ANYBODY.to_uid()))]
    #[case("Another::Authority::Anybody", None)]
    fn lookup_security_provider(#[case] name: impl AsRef<Path>, #[case] uid: Option<Uid>) {
        let name = name.as_ref();
        let stripped = format!("{}::{}", name.table().unwrap(), name.object());
        assert_eq!(self::core::shared::LOOKUP.by_path(name), uid);
        if let Some(uid) = uid {
            assert_eq!(self::core::shared::LOOKUP.by_uid(uid), Some(stripped));
        }
    }

    #[rstest]
    #[case("Authority::Anybody", Some(self::core::shared::authority::ANYBODY.to_uid()))]
    #[case("Shared::Authority::Anybody", Some(self::core::shared::authority::ANYBODY.to_uid()))]
    #[case("Invalid::Authority::Anybody", None)]
    fn lookup_feature_shared(#[case] path: impl AsRef<Path>, #[case] uid: Option<Uid>) {
        let path = path.as_ref();
        let stripped = format!("{}::{}", path.table().unwrap(), path.object());
        assert_eq!(self::core::LOOKUP.by_path(path), uid);
        if let Some(uid) = uid {
            assert_eq!(self::core::LOOKUP.by_uid(uid, None), Some(stripped));
        }
    }

    #[rstest]
    #[case("Authority::Anybody", self::core::shared::authority::ANYBODY.to_uid())]
    #[case("Admin::Authority::PSID", self::psid::admin::authority::PSID.to_uid())]
    fn lookup_global(#[case] name: impl AsRef<Path>, #[case] uid: Uid) {
        let features = [];
        assert_eq!(GLOBAL_LOOKUP.by_path(&features, name), Some(uid));
    }
}
