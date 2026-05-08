//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_packet::{ObjectRef, TableRef, Uid, discovery::FeatureDescriptor};

use crate::path::Path;

// These tables and types are hardcoded here because the generated specification
// is not available here.
pub const SP_TABLE: TableRef = TableRef::new(0x00000205_00000000);
pub const META_TABLE: TableRef = TableRef::new(0xFFFFFFFF_00000000);
pub(crate) type SpRef = ObjectRef<{ SP_TABLE.to_u64() }>;

pub struct NameRange {
    pub prefix: &'static str,
    pub suffix: &'static str,
}

pub struct SortedMap<'a, K, V> {
    pub items: &'a [(K, V)],
}

impl<'a, K, V> SortedMap<'a, K, V> {
    pub fn get(&self, key: &K) -> Option<&V>
    where
        K: Ord + PartialEq,
    {
        self.items.binary_search_by_key(&key, |(k, _)| k).ok().map(|index| &self.items[index].1)
    }
}

pub struct TableIdLookup<'a> {
    pub by_name: SortedMap<'a, &'a str, TableRef>,
    pub by_uid: SortedMap<'a, TableRef, &'a str>,
}

impl<'a> TableIdLookup<'a> {
    pub fn by_name(&self, name: &str) -> Option<TableRef> {
        self.by_name.get(&name).cloned()
    }

    pub fn by_uid(&self, uid: TableRef) -> Option<&'a str> {
        self.by_uid.get(&uid).cloned()
    }
}

pub trait TableLookup {
    fn name(&self) -> &str;
    fn by_path(&self, path: &Path) -> Option<Uid>;
    fn by_name(&self, name: &str) -> Option<Uid>;
    fn by_uid(&self, uid: Uid) -> Option<String>;
}

pub struct ObjectTableLookup<'a, const TABLE: u64> {
    pub name: &'a str,
    pub by_name: SortedMap<'a, &'a str, ObjectRef<TABLE>>,
    pub by_uid: SortedMap<'a, ObjectRef<TABLE>, &'a str>,
}

impl<'a, const TABLE: u64> TableLookup for ObjectTableLookup<'a, TABLE> {
    fn name(&self) -> &str {
        self.name
    }

    fn by_path(&self, path: &Path) -> Option<Uid> {
        if let Some(table_name) = path.table()
            && table_name != self.name
        {
            return None;
        }
        self.by_name(path.object())
    }

    fn by_name(&self, name: &str) -> Option<Uid> {
        self.by_name.get(&name).map(|x| x.to_uid())
    }

    fn by_uid(&self, uid: Uid) -> Option<String> {
        if let Ok(uid) = ObjectRef::<TABLE>::try_from(uid) {
            self.by_uid.get(&uid).map(|name| (*name).to_owned())
        } else {
            None
        }
    }
}

pub struct MetaTableLookup<'a> {
    pub name: &'a str,
    pub by_name: SortedMap<'a, &'a str, Uid>,
    pub by_uid: SortedMap<'a, Uid, &'a str>,
}

impl<'a> TableLookup for MetaTableLookup<'a> {
    fn name(&self) -> &str {
        self.name
    }

    fn by_path(&self, path: &Path) -> Option<Uid> {
        if let Some(table_name) = path.table()
            && table_name != self.name
        {
            return None;
        }
        self.by_name(path.object())
    }

    fn by_name(&self, name: &str) -> Option<Uid> {
        self.by_name.get(&name).cloned()
    }

    fn by_uid(&self, uid: Uid) -> Option<String> {
        self.by_uid.get(&uid).map(|name| (*name).to_owned())
    }
}

pub struct SecurityProviderLookup<'a> {
    pub name: &'a str,
    pub table_ids: &'a TableIdLookup<'a>,
    pub table_lookups: SortedMap<'a, TableRef, &'a dyn TableLookup>,
}

impl<'a> SecurityProviderLookup<'a> {
    pub fn by_path(&self, path: impl AsRef<Path>) -> Option<Uid> {
        let path = path.as_ref();
        if let Some(sp_name) = path.security_provider()
            && sp_name != self.name
        {
            return None;
        }
        let table_ref = self.table_ids.by_name(path.table()?)?;
        self.by_path_with_table(path.object(), table_ref)
    }

    pub fn by_path_with_table(&self, name: &str, table: TableRef) -> Option<Uid> {
        if table == META_TABLE {
            for (table_ref, table_lookup) in self.table_lookups.items.iter().rev() {
                if *table_ref == META_TABLE {
                    if let Some(uid) = table_lookup.by_name(name) {
                        return Some(uid);
                    }
                } else {
                    break;
                }
            }
            return None;
        } else {
            let table_lookup = self.table_lookups.get(&table)?;
            table_lookup.by_name(name)
        }
    }

    pub fn by_uid(&self, uid: Uid) -> Option<String> {
        let table_ref = uid.containing_table().map(|table_ref| TableRef::new(table_ref.to_u64()));
        let path = if let Some(table_ref) = table_ref {
            let table_lookup = self.table_lookups.get(&table_ref)?;
            let table_name = table_lookup.name();
            table_lookup.by_uid(uid).map(|name| format!("{table_name}::{name}"))
        } else {
            let mut meta_tables =
                self.table_lookups.items.iter().rev().filter(|(table_ref, _)| table_ref == &META_TABLE);
            meta_tables.find_map(|(_, lookup)| lookup.by_uid(uid).map(|name| format!("{}::{name}", lookup.name())))
        };
        match self.name {
            "Shared" => path,
            sp => path.map(|path| format!("{sp}::{path}")),
        }
    }
}

pub struct FeatureLookup<'a> {
    pub sp_lookups: SortedMap<'a, Option<SpRef>, &'a SecurityProviderLookup<'a>>,
}

impl<'a> FeatureLookup<'a> {
    pub fn by_path(&self, path: impl AsRef<Path>) -> Option<Uid> {
        let path = path.as_ref();
        let sp_ref = if let Some(sp_name) = path.security_provider() {
            if sp_name == "Shared" {
                None
            } else {
                let sp_path = format!("SP::{sp_name}");
                let sp_ref = self.sp_lookups.items.iter().find_map(|(_, sp_lookup)| sp_lookup.by_path(&sp_path));
                Some(sp_ref?)
            }
        } else {
            None
        };
        let sp_ref = sp_ref.map(|uid| uid.try_into().expect("lookup must return an SP"));
        self.by_path_with_sp(path, sp_ref)
    }

    pub fn by_path_with_sp(&self, path: impl AsRef<Path>, sp: Option<SpRef>) -> Option<Uid> {
        let sp_lookup = self.sp_lookups.get(&sp)?;
        sp_lookup.by_path(path)
    }

    pub fn by_path_with_sp_and_table(&self, name: &str, sp: Option<SpRef>, table: TableRef) -> Option<Uid> {
        let sp_lookup = self.sp_lookups.get(&sp)?;
        sp_lookup.by_path_with_table(name, table)
    }

    pub fn by_uid(&self, uid: Uid, sp: Option<SpRef>) -> Option<String> {
        let sp_lookup = self.sp_lookups.get(&sp)?;
        sp_lookup.by_uid(uid)
    }
}

/// This implementation is kinda hacky:
/// - This is `lookup.rs`, but it imports `preconfig.rs`, which is based on
///   `lookup.rs`. This compiles in Rust, but is not pretty.
/// - The features are hardcoded. Instead, they should be specified dynamically
///   based on the JSON specification. This may be possible once the discovery
///   module is fully cleaned up, but that needs all the refactoring.
pub struct GlobalLookup;

impl GlobalLookup {
    pub fn by_path(&self, features: &[FeatureDescriptor], path: impl AsRef<Path>) -> Option<Uid> {
        let path = path.as_ref();
        self.search(features, |lookup| lookup.by_path(path))
    }

    pub fn by_path_with_sp(
        &self,
        features: &[FeatureDescriptor],
        path: impl AsRef<Path>,
        sp: Option<SpRef>,
    ) -> Option<Uid> {
        let path = path.as_ref();
        self.search(features, |lookup| lookup.by_path_with_sp(path, sp))
    }

    pub fn by_path_with_sp_and_table(
        &self,
        features: &[FeatureDescriptor],
        name: &str,
        sp: Option<SpRef>,
        table: TableRef,
    ) -> Option<Uid> {
        self.search(features, |lookup| lookup.by_path_with_sp_and_table(name, sp, table))
    }

    pub fn by_uid(&self, features: &[FeatureDescriptor], uid: Uid, sp: Option<SpRef>) -> Option<String> {
        self.search(features, |lookup| lookup.by_uid(uid, sp))
    }

    fn search<T>(&self, features: &[FeatureDescriptor], f: impl Fn(FeatureLookup) -> Option<T>) -> Option<T> {
        use crate::preconfig;
        let ssc_features = features.iter().filter(|feature| feature.security_subsystem_class().is_some());
        let common_features = features.iter().filter(|feature| feature.security_subsystem_class().is_some());
        for feature in ssc_features {
            let result = match feature {
                FeatureDescriptor::Enterprise(_) => f(preconfig::enterprise::LOOKUP),
                FeatureDescriptor::OpalV1(_) => f(preconfig::opal_2::LOOKUP),
                FeatureDescriptor::OpalV2(_) => f(preconfig::opal_2::LOOKUP),
                FeatureDescriptor::Opalite(_) => f(preconfig::opalite::LOOKUP),
                FeatureDescriptor::PyriteV1(_) => f(preconfig::pyrite_2::LOOKUP),
                FeatureDescriptor::PyriteV2(_) => f(preconfig::pyrite_2::LOOKUP),
                FeatureDescriptor::Ruby(_) => f(preconfig::ruby::LOOKUP),
                FeatureDescriptor::KeyPerIO(_) => f(preconfig::kpio::LOOKUP),
                _ => None,
            };
            if result.is_some() {
                return result;
            }
        }
        for feature in common_features {
            let result = match feature {
                FeatureDescriptor::AdditionalDataStoreTables(_) => f(preconfig::data_store::LOOKUP),
                _ => None,
            };
            if result.is_some() {
                return result;
            }
        }
        if let Some(result) = f(preconfig::psid::LOOKUP) {
            return Some(result);
        }
        if let Some(result) = f(preconfig::core::LOOKUP) {
            return Some(result);
        }
        None
    }
}
