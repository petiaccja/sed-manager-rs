//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

//! The specification is generated from a JSON description with a build.rs script.
//!
//! This avoids code bloat for the actual Rust code and makes editing the specification
//! much easier.

#![allow(unused)]

// The generated Rust code uses ONLY the entities declared below.
// crate::* imports are PROHIBITED in the generated code. This method creates
// a protective layer so the rest of the library can be refactored without
// introducing any modifications to the code generator.
use crate::messaging::table_mask::table_mask;
use crate::messaging::uid::ObjectUID;
use crate::messaging::uid::TableUID;
use crate::messaging::uid::UID;
use crate::messaging::uid_range::ObjectUIDRange;
use crate::messaging::uid_range::UIDRange;
use crate::spec::generated as root;
use crate::spec::lookup;

// This inlines the generated code.
include!(concat!(env!("OUT_DIR"), "/spec.rs"));

#[cfg(test)]
mod tests {
    use super::core::all::table_id::TABLE_LOOKUP;
    use crate::spec::{lookup::TableLookup, ObjectLookup};

    use super::*;

    #[test]
    fn lookup_table_with_object() {
        let expected = Some((core::all::table_id::AUTHORITY.as_uid(), "SID"));
        let result = TABLE_LOOKUP.resolve("Authority::SID");
        assert_eq!(result, expected);
    }

    #[test]
    fn lookup_table_no_object() {
        let expected = Some((core::all::table_id::AUTHORITY.as_uid(), ""));
        let result = TABLE_LOOKUP.resolve("Authority");
        assert_eq!(result, expected);
    }

    #[test]
    fn lookup_table_non_existing() {
        let expected = None;
        let result = TABLE_LOOKUP.resolve("DoesNotExist::Object");
        assert_eq!(result, expected);
    }

    #[test]
    fn lookup_object_core_no_sp() {
        let uid = core::all::table::CRYPTO_SUITE.as_uid();
        let name = "Table::CryptoSuite";
        assert_eq!(core::OBJECT_LOOKUP.by_path(name, None), Some(uid));
        assert_eq!(core::OBJECT_LOOKUP.by_uid(uid, None), Some("CryptoSuite".to_string()));
    }

    #[test]
    fn lookup_object_core_with_sp() {
        let sp = opal_2::admin::sp::ADMIN.as_uid();
        let uid = core::all::table::CRYPTO_SUITE.as_uid();
        let name = "Table::CryptoSuite";
        assert_eq!(core::OBJECT_LOOKUP.by_path(name, Some(sp)), Some(uid));
        assert_eq!(core::OBJECT_LOOKUP.by_uid(uid, Some(sp)), Some("CryptoSuite".to_string()));
    }

    #[test]
    fn lookup_object_core_bad_object() {
        let sp = opal_2::admin::sp::ADMIN.as_uid();
        let uid = UID::new(0x234789253478334);
        let name = "Table::BadObject";
        assert_eq!(core::OBJECT_LOOKUP.by_path(name, Some(sp)), None);
        assert_eq!(core::OBJECT_LOOKUP.by_uid(uid, Some(sp)), None);
    }

    #[test]
    fn lookup_object_ssc_no_sp() {
        let uid = opal_2::admin::c_pin::MSID.as_uid();
        let name = "C_PIN::MSID";
        assert_eq!(opal_2::OBJECT_LOOKUP.by_path(name, None), None);
        assert_eq!(opal_2::OBJECT_LOOKUP.by_uid(uid, None), None);
    }

    #[test]
    fn lookup_object_ssc_with_good_sp() {
        let sp = opal_2::admin::sp::ADMIN.as_uid();
        let uid = opal_2::admin::c_pin::MSID.as_uid();
        let name = "C_PIN::MSID";
        assert_eq!(opal_2::OBJECT_LOOKUP.by_path(name, Some(sp)), Some(uid));
        assert_eq!(opal_2::OBJECT_LOOKUP.by_uid(uid, Some(sp)), Some("MSID".to_string()));
    }

    #[test]
    fn lookup_object_ssc_with_bad_sp() {
        let sp = opal_2::admin::sp::LOCKING.as_uid();
        let uid = opal_2::admin::c_pin::MSID.as_uid();
        let name = "C_PIN::MSID";
        assert_eq!(opal_2::OBJECT_LOOKUP.by_path(name, Some(sp)), None);
        assert_eq!(opal_2::OBJECT_LOOKUP.by_uid(uid, Some(sp)), None);
    }

    #[test]
    fn lookup_object_range() {
        let sp = opal_2::admin::sp::LOCKING.as_uid();
        let uid = opal_2::locking::c_pin::USER.nth(7).unwrap().as_uid();
        let name = "C_PIN::User7";
        assert_eq!(opal_2::OBJECT_LOOKUP.by_path(name, Some(sp)), Some(uid));
        assert_eq!(opal_2::OBJECT_LOOKUP.by_uid(uid, Some(sp)), Some("User7".into()));
    }
}
