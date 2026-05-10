//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::{collections::HashMap, str::FromStr as _};

use proc_macro2;
use quote::{format_ident, quote};
use serde::{Deserialize, Deserializer, de::Error as _};
use syn::{Expr, File, Ident, ItemConst, ItemMod, parse_quote};

type Error = Box<dyn std::error::Error>;

#[derive(Debug)]
struct MessageError(String);

impl std::fmt::Display for MessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for MessageError {}

#[derive(PartialEq, Eq)]
enum CharType {
    None,
    Lower,
    Upper,
}

impl From<char> for CharType {
    fn from(value: char) -> Self {
        if value.is_lowercase() {
            Self::Lower
        } else if value.is_uppercase() {
            Self::Upper
        } else {
            Self::None
        }
    }
}

fn pascal_case_to_snake_case(id: &str) -> String {
    use CharType::*;
    let mut window = [None, None, None];
    let mut out = String::new();
    for ch in id.chars() {
        window.rotate_left(1);
        window[2] = ch.into();
        if window[1..] == [Lower, Upper] {
            out.push('_');
        } else if window == [Upper, Upper, Lower] {
            let last = out.pop();
            out.push('_');
            last.into_iter().for_each(|ch| out.push(ch));
        }
        out.push(ch);
    }
    out
}

fn to_mod_ident(id: &str) -> Ident {
    format_ident!("{}", pascal_case_to_snake_case(id).to_lowercase())
}

fn to_const_ident(id: &str) -> Ident {
    format_ident!("{}", pascal_case_to_snake_case(id).to_uppercase())
}

fn one() -> u64 {
    1
}

fn deserialize_uid<'de, D>(de: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    u64::from_str_radix(&String::deserialize(de)?, 16).map_err(|_| D::Error::custom("could not parse UID"))
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
enum Object {
    Unique(#[serde(deserialize_with = "deserialize_uid")] u64),
    Range {
        #[serde(deserialize_with = "deserialize_uid")]
        base: u64,
        count: u64,
        #[serde(default = "one")]
        step: u64,
        display_base: u64,
    },
}

impl Object {
    pub fn generate(&self, name: &str) -> Result<ItemConst, Error> {
        let name = name.replace("{n}", "").replace("__", "_");
        let ident = to_const_ident(&name);
        let base_hex = self.base_hex();
        match self {
            Object::Unique(_) => Ok(parse_quote! {
                pub const #ident : ObjectRef<{THIS_TABLE.to_u64()}> = ObjectRef::new(#base_hex);
            }),
            Object::Range { count, step, .. } => Ok(parse_quote! {
                pub const #ident : UidRange<ObjectRef<{THIS_TABLE.to_u64()}>> = UidRange {
                    start: ObjectRef::new(#base_hex),
                    end: ObjectRef::new(#base_hex + #count * #step),
                    step: #step as u32,
                };
            }),
        }
    }

    pub fn base(&self) -> u64 {
        match self {
            Object::Unique(base) => *base,
            Object::Range { base, .. } => *base,
        }
    }

    pub fn base_hex(&self) -> proc_macro2::Literal {
        let base = self.base();
        proc_macro2::Literal::from_str(&format!("0x{base:016x}_u64")).unwrap()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct Table(HashMap<String, Object>);

impl Table {
    pub fn generate(&self, name: &str) -> Result<ItemMod, Error> {
        let mod_name = to_mod_ident(name);
        if name == "TableID" {
            self.generate_table_id(&mod_name)
        } else if name == "General" || name == "InvokingID" || name == "SMMethodID" {
            self.generate_meta_table(&mod_name, name)
        } else {
            self.generate_object_table(mod_name, name)
        }
    }

    fn generate_table_id(&self, mod_name: &Ident) -> Result<ItemMod, Error> {
        let objects = self.0.iter().filter_map(|(name, object)| {
            if let Object::Unique(_) = object {
                let ident = to_const_ident(name);
                let base_hex = object.base_hex();
                Some(quote! { pub const #ident : TableRef = TableRef::new(#base_hex); })
            } else {
                None
            }
        });

        let by_name: Vec<_> = self.by_name_unique();
        let by_uid: Vec<_> = self.by_uid_unique();
        let len = by_name.len();

        Ok(parse_quote! {
            pub mod #mod_name {
                use ::sed_packet::{TableRef};

                #(#objects)*

                const BY_NAME: [(&str, TableRef); #len] = [
                    #(#by_name),*
                ];

                const BY_UID: [(TableRef, &str); #len] = [
                    #(#by_uid),*
                ];

                pub const LOOKUP: crate::lookup::TableIdLookup<'static> = crate::lookup::TableIdLookup{
                    by_name: crate::lookup::SortedMap{ items: &BY_NAME },
                    by_uid: crate::lookup::SortedMap{ items: &BY_UID },
                };
            }
        })
    }

    fn generate_meta_table(&self, mod_name: &Ident, name: &str) -> Result<ItemMod, Error> {
        let table_ref = to_const_ident(name);
        let objects = self.0.iter().map(|(name, object)| {
            if let Object::Unique(base) = object {
                let object_name = to_const_ident(name);
                let base_hex = proc_macro2::Literal::from_str(&format!("0x{base:016x}_u64")).unwrap();
                quote! { pub const #object_name : Uid = Uid::new(#base_hex); }
            } else {
                quote! {}
            }
        });

        let by_name: Vec<_> = self.by_name_unique();
        let by_uid: Vec<_> = self.by_uid_unique();
        let len = by_name.len();

        Ok(parse_quote! {
            pub mod #mod_name {
                use ::sed_packet::{TableRef, Uid};

                const THIS_TABLE : TableRef = super::super::super::core::shared::table_id::#table_ref;

                #(#objects)*

                const BY_NAME: [(&str, Uid); #len] = [
                    #(#by_name),*
                ];

                const BY_UID: [(Uid, &str); #len] = [
                    #(#by_uid),*
                ];

                pub const LOOKUP: crate::lookup::MetaTableLookup<'static> = crate::lookup::MetaTableLookup{
                    name: #name,
                    by_name: crate::lookup::SortedMap{ items: &BY_NAME },
                    by_uid: crate::lookup::SortedMap{ items: &BY_UID },
                };
            }
        })
    }

    fn generate_object_table(&self, mod_name: Ident, name: &str) -> Result<ItemMod, Error> {
        let table_ref = to_const_ident(name);
        let objects = self.0.iter().map(|(name, object)| object.generate(name)).collect::<Result<Vec<_>, _>>()?;

        let by_name: Vec<_> = self.by_name_unique();
        let by_uid: Vec<_> = self.by_uid_unique();
        let len = by_name.len();

        Ok(parse_quote! {
            pub mod #mod_name {
                use ::sed_packet::{TableRef, ObjectRef, UidRange};

                const THIS_TABLE : TableRef = super::super::super::core::shared::table_id::#table_ref;

                #(#objects)*

                const BY_NAME: [(&str, ObjectRef<{THIS_TABLE.to_u64()}>); #len] = [
                    #(#by_name),*
                ];

                const BY_UID: [(ObjectRef<{THIS_TABLE.to_u64()}>, &str); #len] = [
                    #(#by_uid),*
                ];

                pub const LOOKUP: crate::lookup::ObjectTableLookup<'static, {THIS_TABLE.to_u64()}> = crate::lookup::ObjectTableLookup{
                    name: #name,
                    by_name: crate::lookup::SortedMap{ items: &BY_NAME },
                    by_uid: crate::lookup::SortedMap{ items: &BY_UID },
                };
            }
        })
    }

    fn by_name_unique(&self) -> Vec<Expr> {
        let mut objects: Vec<_> = self
            .0
            .iter()
            .filter_map(|(name, object)| matches!(object, Object::Unique(_)).then_some(name))
            .collect();
        objects.sort();
        objects
            .iter()
            .map(|name| {
                let ident = to_const_ident(name);
                parse_quote! { (#name, #ident) }
            })
            .collect()
    }

    fn by_uid_unique(&self) -> Vec<Expr> {
        let mut objects: Vec<_> = self
            .0
            .iter()
            .filter_map(|(name, object)| match object {
                Object::Unique(base) => Some((*base, name)),
                _ => None,
            })
            .collect();
        objects.sort();
        objects
            .iter()
            .map(|(_, name)| {
                let ident = to_const_ident(name);
                parse_quote! { (#ident, #name) }
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct SecurityProvider(HashMap<String, Table>);

impl SecurityProvider {
    pub fn generate(&self, name: &str, table_id: &HashMap<&str, u64>) -> Result<ItemMod, Error> {
        let mod_name = to_mod_ident(name);
        let tables = self.0.iter().map(|(name, table)| table.generate(name)).collect::<Result<Vec<_>, _>>()?;

        let sorted_tables = self.sorted_tables(table_id)?;
        let lookups = sorted_tables.iter().map(|name| {
            let table_uid_ident = to_const_ident(name);
            let table_mod_ident = to_mod_ident(name);
            quote! { (super::super::core::shared::table_id::#table_uid_ident, &#table_mod_ident::LOOKUP) }
        });
        let len = lookups.len();

        Ok(parse_quote!(
            pub mod #mod_name {
                use sed_packet::TableRef;

                #(#tables)*

                const TABLE_LOOKUPS: [(TableRef, &dyn crate::lookup::TableLookup); #len] = [
                    #(#lookups),*
                ];

                pub const LOOKUP: crate::lookup::SecurityProviderLookup<'static> = crate::lookup::SecurityProviderLookup{
                    name: #name,
                    table_ids: &super::super::core::shared::table_id::LOOKUP,
                    table_lookups: crate::lookup::SortedMap{ items: &TABLE_LOOKUPS },
                };
            }
        ))
    }

    pub fn sorted_tables(&self, table_id: &HashMap<&str, u64>) -> Result<Vec<&str>, Error> {
        let mut table_pairs: Vec<(&str, u64)> = self
            .0
            .keys()
            .filter_map(|name| {
                if name != "TableID" {
                    Some(
                        table_id
                            .get(name.as_str())
                            .ok_or_else(|| {
                                MessageError(format!("table id not found for security provider table '{name}'"))
                            })
                            .map(|id| (name.as_str(), *id)),
                    )
                } else {
                    None
                }
            })
            .collect::<Result<_, _>>()?;

        table_pairs.sort_by_key(|(_, id)| *id);

        let sorted_names = table_pairs.into_iter().map(|(name, _)| name).collect();

        Ok(sorted_names)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct Feature(HashMap<String, SecurityProvider>);

impl Feature {
    pub fn generate(&self, name: &str, table_id: &HashMap<&str, u64>) -> Result<ItemMod, Error> {
        let mod_name = to_mod_ident(name);
        let security_providers = self
            .0
            .iter()
            .map(|(name, security_provider)| security_provider.generate(name, table_id))
            .collect::<Result<Vec<ItemMod>, _>>()?;

        let sorted_sps = self.sorted_sps();
        let lookups = sorted_sps.iter().map(|name| {
            let const_ident = to_const_ident(name);
            let mod_ident = to_mod_ident(name);
            if *name == "Shared" {
                quote! { (None, &#mod_ident::LOOKUP) }
            } else {
                quote! { (Some(admin::sp::#const_ident), &#mod_ident::LOOKUP) }
            }
        });
        let len = lookups.len();

        Ok(parse_quote!(
            pub mod #mod_name {
                use crate::lookup::SpRef;

                #(#security_providers)*

                const SP_LOOKUPS: [(Option<SpRef>, &crate::lookup::SecurityProviderLookup); #len] = [
                    #(#lookups),*
                ];

                pub const LOOKUP: crate::lookup::FeatureLookup<'static> = crate::lookup::FeatureLookup{
                    sp_lookups: crate::lookup::SortedMap{ items: &SP_LOOKUPS },
                };
            }
        ))
    }

    pub fn sorted_sps(&self) -> Vec<&str> {
        let mut sps = Vec::new();
        if self.0.contains_key("Shared") {
            sps.push(("Shared", 0));
        }
        if let Some(admin_sp) = self.0.get("Admin") {
            if let Some(sp_table) = admin_sp.0.get("SP") {
                for (name, uid) in &sp_table.0 {
                    sps.push((name.as_str(), uid.base()));
                }
            }
        }
        sps.sort_by_key(|(_, uid)| *uid);
        sps.iter().map(|(name, _)| *name).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct Spec(HashMap<String, Feature>);

impl Spec {
    pub fn generate(&self) -> Result<File, Error> {
        let mut table_id = HashMap::new();
        for (_, feature) in &self.0 {
            for (_, sp) in &feature.0 {
                for (table_name, table) in &sp.0 {
                    if table_name == "TableID" {
                        let tables = table.0.iter().filter_map(|(name, object)| match object {
                            Object::Unique(base) => Some((*base, name)),
                            _ => None,
                        });
                        for (base, name) in tables {
                            table_id.insert(name.as_str(), base);
                        }
                    }
                }
            }
        }
        let features = self
            .0
            .iter()
            .map(|(name, feature)| feature.generate(name, &table_id))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(parse_quote!(
            #(#features)*
        ))
    }
}

fn generate_file(spec: &Spec) -> Result<File, Error> {
    let spec = spec.generate()?;
    Ok(parse_quote!(
        const GENERATED_MARKER: () = ();

        #spec
    ))
}

pub fn generate_spec(spec: &str) -> Result<String, Error> {
    let spec: Spec = serde_json::from_str(spec)?;
    let file = generate_file(&spec)?;
    Ok(prettyplease::unparse(&file))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SPEC: &str = r#"
    {
        "Core": {
            "Shared": {
                "TableID": {
                    "Authority": "0000000900000000"
                }
            }
        },
        "Opal_2": {
            "Admin": {
                "Authority": {
                    "SID": "0000000900000006",
                    "Admin{n}": {
                        "base": "0000000900000201",
                        "count": 256,
                        "step": 1,
                        "display_base": 1
                    }
                }
            }
        }
    }
    "#;

    #[test]
    fn parse_example() {
        let expected = Spec(HashMap::from([
            (
                "Core".into(),
                Feature(HashMap::from([(
                    "Shared".into(),
                    SecurityProvider(HashMap::from([(
                        "TableID".into(),
                        Table(HashMap::from([("Authority".into(), Object::Unique(0x0000000900000000))])),
                    )])),
                )])),
            ),
            (
                "Opal_2".into(),
                Feature(HashMap::from([(
                    "Admin".into(),
                    SecurityProvider(HashMap::from([(
                        "Authority".into(),
                        Table(HashMap::from([
                            ("SID".into(), Object::Unique(0x0000000900000006)),
                            (
                                "Admin{n}".into(),
                                Object::Range { base: 0x0000000900000201, count: 256, step: 1, display_base: 1 },
                            ),
                        ])),
                    )])),
                )])),
            ),
        ]));
        assert_eq!(serde_json::from_str::<Spec>(TEST_SPEC).unwrap(), expected);
    }

    #[test]
    fn generate_example() {
        let _spec = generate_spec(TEST_SPEC).unwrap();
    }
}
