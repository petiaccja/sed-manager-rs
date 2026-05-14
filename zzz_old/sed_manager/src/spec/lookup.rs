//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::messaging::{uid::UID, uid_range::UIDRange};

pub struct NameRange {
    pub prefix: &'static str,
    pub suffix: &'static str,
}

pub struct Path<'buffer> {
    pub table: &'buffer str,
    pub object: Option<&'buffer str>,
}

pub trait TableLookup {
    fn resolve<'path>(&self, path: &'path str) -> Option<(UID, &'path str)>;
}

pub trait ObjectLookup {
    fn by_uid(&self, uid: UID, sp: Option<UID>) -> Option<String>;
    fn by_name(&self, name: &str, table: UID, sp: Option<UID>) -> Option<UID>;
    fn by_path(&self, path: &str, sp: Option<UID>) -> Option<UID>;
    fn has_table(&self, #[allow(unused)] table: UID) -> bool {
        true
    }
    fn has_sp(&self, #[allow(unused)] sp: UID) -> bool {
        true
    }
}

pub struct ListTableLookup<const TABLE_COUNT: usize> {
    pub uids_by_name: [(&'static str, UID); TABLE_COUNT],
}

pub struct CompositeObjectLookup<const SP: u64, const CHILD_COUNT: usize> {
    pub table_lookup: &'static dyn TableLookup,
    pub children: [&'static dyn ObjectLookup; CHILD_COUNT],
}

pub struct ListObjectLookup<const TABLE: u64, const UID_COUNT: usize, const RANGE_COUNT: usize> {
    pub table_lookup: &'static dyn TableLookup,
    pub uids_by_value: [(UID, &'static str); UID_COUNT],
    pub ranges_by_value: [(UIDRange, NameRange); RANGE_COUNT],
    pub uids_by_name: [(&'static str, UID); UID_COUNT],
    pub ranges_by_name: [(NameRange, UIDRange); RANGE_COUNT],
}

impl<const TABLE_COUNT: usize> TableLookup for ListTableLookup<TABLE_COUNT> {
    fn resolve<'path>(&self, path: &'path str) -> Option<(UID, &'path str)> {
        let path = Path::from(path);
        if let Some(idx) = self.uids_by_name.binary_search_by_key(&path.table, |x| x.0).ok() {
            Some((self.uids_by_name[idx].1.clone(), path.object.unwrap_or(&"")))
        } else {
            None
        }
    }
}

impl<const SP: u64, const CHILD_COUNT: usize> ObjectLookup for CompositeObjectLookup<SP, CHILD_COUNT> {
    fn by_uid(&self, uid: UID, sp: Option<UID>) -> Option<String> {
        let table = uid.containing_table().unwrap_or(UID::null());
        for child in &self.children {
            if child.has_table(table) && child.has_sp(sp.unwrap_or(UID::null())) {
                if let Some(name) = child.by_uid(uid, sp) {
                    return Some(name);
                }
            }
        }
        None
    }

    fn by_path(&self, path: &str, sp: Option<UID>) -> Option<UID> {
        self.table_lookup
            .resolve(path)
            .map(|(table, object)| self.by_name(object, table, sp))
            .unwrap_or(None)
    }

    fn by_name(&self, object: &str, table: UID, sp: Option<UID>) -> Option<UID> {
        for child in &self.children {
            if child.has_table(table) && child.has_sp(sp.unwrap_or(UID::null())) {
                if let Some(name) = child.by_name(object, table, sp) {
                    return Some(name);
                }
            }
        }
        None
    }

    fn has_table(&self, table: UID) -> bool {
        self.children.iter().any(|child| child.has_table(table))
    }

    fn has_sp(&self, sp: UID) -> bool {
        let this_sp = UID::new(SP);
        if this_sp == UID::null() || this_sp == sp {
            self.children.iter().any(|child| child.has_sp(sp))
        } else {
            false
        }
    }
}

impl<const TABLE: u64, const UID_COUNT: usize, const RANGE_COUNT: usize> ObjectLookup
    for ListObjectLookup<TABLE, UID_COUNT, RANGE_COUNT>
{
    fn by_uid(&self, uid: UID, _sp: Option<UID>) -> Option<String> {
        if let Some(&(_, name)) = self.find_uid(uid) {
            Some(name.into())
        } else if let Some((uids, names)) = self.find_uid_range(uid) {
            Some(names.format(uids.index_of(uid).unwrap()))
        } else {
            None
        }
    }

    fn by_name(&self, name: &str, table: UID, _sp: Option<UID>) -> Option<UID> {
        if self.has_table(table) {
            if let Some((_, uid)) = self.find_name(name) {
                Some(uid)
            } else if let Some((_, uids, n)) = self.find_name_range(name) {
                uids.nth(n)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn by_path(&self, name: &str, sp: Option<UID>) -> Option<UID> {
        self.table_lookup
            .resolve(name)
            .map(|(table, object)| self.by_name(object, table, sp))
            .unwrap_or(None)
    }

    fn has_table(&self, table: UID) -> bool {
        TABLE == 0 || table.as_u64() == TABLE
    }
}

impl<const TABLE: u64, const UID_COUNT: usize, const RANGE_COUNT: usize>
    ListObjectLookup<TABLE, UID_COUNT, RANGE_COUNT>
{
    fn find_uid(&self, uid: UID) -> Option<&(UID, &str)> {
        self.uids_by_value
            .binary_search_by_key(&uid, |(uid, _)| *uid)
            .map(|idx| &self.uids_by_value[idx])
            .ok()
    }

    fn find_uid_range(&self, uid: UID) -> Option<&(UIDRange, NameRange)> {
        self.ranges_by_value.iter().find(|(range, _)| range.contains(uid))
    }

    fn find_name(&self, name: &str) -> Option<(&str, UID)> {
        self.uids_by_name
            .binary_search_by_key(&name, |(name, _)| name)
            .map(|idx| self.uids_by_name[idx])
            .ok()
    }

    fn find_name_range(&self, name: &str) -> Option<(&NameRange, &UIDRange, u64)> {
        fn matches(s: &str, range: &NameRange) -> Option<u64> {
            if s.starts_with(range.prefix) && s.ends_with(range.suffix) {
                let middle = &s[range.prefix.len()..(s.len() - range.suffix.len())];
                middle.parse().ok()
            } else {
                None
            }
        }
        self.ranges_by_name
            .iter()
            .find_map(|(names, uids)| matches(name, names).map(|idx| (names, uids, idx)))
    }
}

impl NameRange {
    pub fn format(&self, n: u64) -> String {
        let mut s = self.prefix.to_string();
        s.push_str(&n.to_string());
        s.push_str(&self.suffix);
        s
    }
}

impl<'buffer> From<&'buffer str> for Path<'buffer> {
    fn from(value: &'buffer str) -> Self {
        if let Some(split_idx) = value.find("::") {
            Self { table: &value[..split_idx], object: Some(&value[(split_idx + 2)..]) }
        } else {
            Self { table: value, object: None }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RESOLVER: ListTableLookup<1> = ListTableLookup { uids_by_name: [("Tab", UID::null())] };

    const LOOKUP: ListObjectLookup<0, 2, 2> = ListObjectLookup {
        table_lookup: &RESOLVER,
        uids_by_value: [(UID::new(1), "One"), (UID::new(2), "Two")],
        uids_by_name: [("One", UID::new(1)), ("Two", UID::new(2))],
        ranges_by_value: [
            (UIDRange::new_count(UID::new(100), 10, 1), NameRange { prefix: "Hundred", suffix: "_" }),
            (UIDRange::new_count(UID::new(200), 10, 1), NameRange { prefix: "TwoHundred", suffix: "_" }),
        ],
        ranges_by_name: [
            (NameRange { prefix: "Hundred", suffix: "_" }, UIDRange::new_count(UID::new(100), 10, 1)),
            (NameRange { prefix: "TwoHundred", suffix: "_" }, UIDRange::new_count(UID::new(200), 10, 1)),
        ],
    };

    #[test]
    fn by_uid_unique() {
        assert_eq!(LOOKUP.by_uid(UID::new(0), None), None);
        assert_eq!(LOOKUP.by_uid(UID::new(1), None), Some("One".into()));
        assert_eq!(LOOKUP.by_uid(UID::new(2), None), Some("Two".into()));
        assert_eq!(LOOKUP.by_uid(UID::new(3), None), None);
    }

    #[test]
    fn by_uid_range() {
        assert_eq!(LOOKUP.by_uid(UID::new(104), None), Some("Hundred4_".into()));
        assert_eq!(LOOKUP.by_uid(UID::new(199), None), None);
        assert_eq!(LOOKUP.by_uid(UID::new(200), None), Some("TwoHundred0_".into()));
        assert_eq!(LOOKUP.by_uid(UID::new(209), None), Some("TwoHundred9_".into()));
        assert_eq!(LOOKUP.by_uid(UID::new(210), None), None);
    }

    #[test]
    fn by_name_unique() {
        assert_eq!(LOOKUP.by_path("Missing::One".into(), None), None);
        assert_eq!(LOOKUP.by_path("Tab::Aaa".into(), None), None);
        assert_eq!(LOOKUP.by_path("Tab::One".into(), None), Some(UID::new(1)));
        assert_eq!(LOOKUP.by_path("Tab::Two".into(), None), Some(UID::new(2)));
        assert_eq!(LOOKUP.by_path("Tab::Zzz".into(), None), None);
    }

    #[test]
    fn by_name_range() {
        assert_eq!(LOOKUP.by_path("Tab::Hundred4_".into(), None), Some(UID::new(104)));
        assert_eq!(LOOKUP.by_path("Tab::TwoHundred0_".into(), None), Some(UID::new(200)));
        assert_eq!(LOOKUP.by_path("Tab::TwoHundred9_".into(), None), Some(UID::new(209)));
        assert_eq!(LOOKUP.by_path("Tab::TwoHundred00009_".into(), None), Some(UID::new(209)));
        assert_eq!(LOOKUP.by_path("Tab::TwoHundred10_".into(), None), None);
        assert_eq!(LOOKUP.by_path("Tab::TwoHundred10".into(), None), None);
        assert_eq!(LOOKUP.by_path("Tab::TwoHundrea10_".into(), None), None);
        assert_eq!(LOOKUP.by_path("Tab::TwoHundrez10_".into(), None), None);
    }
}
