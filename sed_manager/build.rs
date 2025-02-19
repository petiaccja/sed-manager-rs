//! Generates Rust code for the specification defined in JSON.
//!
//! Important:
//! - Generated code should never use `crate::`` imports.
//! - Generated code should access all dependencies via `root::`` imports.
//! - `root::` is defined in the module that `include!`s the generated code.

use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use serde_json::Value;
use std::{fs::File, io::Write, path::PathBuf};

#[derive(Debug)]
#[allow(unused)]
enum ParseError {
    InvalidFile,
    InvalidFeature(String),
    InvalidSecurityProvider(String),
    InvalidTable(String),
    InvalidUIDFormat(String),
    InvalidUIDBase(String),
    InvalidUIDCount(String),
    InvalidUIDStep(String),
}

fn is_range(name: &str) -> bool {
    name.contains('{') || name.contains('}')
}

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

fn to_mod_identifier(id: &str) -> proc_macro2::Ident {
    let out = pascal_case_to_snake_case(id);
    format_ident!("{}", out.to_lowercase())
}

fn to_const_identifier(id: &str) -> proc_macro2::Ident {
    let out = pascal_case_to_snake_case(id);
    format_ident!("{}", out.to_uppercase())
}

struct Feature {
    name: String,
    security_providers: Vec<SecurityProvider>,
}

struct SecurityProvider {
    name: String,
    tables: Vec<Table>,
}

struct Table {
    name: String,
    uids: Vec<UID>,
    uid_ranges: Vec<UIDRange>,
}

#[derive(Clone)]
struct UID {
    name: String,
    base: u64,
}

#[derive(Clone)]
struct UIDRange {
    name: String,
    base: u64,
    count: u64,
    step: u64,
}

impl Feature {
    fn parse(name: String, data: &Value) -> Result<Self, ParseError> {
        let security_providers = match data {
            Value::Object(security_providers) => security_providers
                .iter()
                .map(|(name, value)| SecurityProvider::parse(name.clone(), value))
                .collect::<Result<Vec<_>, _>>(),
            _ => Err(ParseError::InvalidFeature(name.clone())),
        }?;
        Ok(Self { name, security_providers })
    }

    fn generate_lookup(&self) -> TokenStream2 {
        let child_count = self.security_providers.len();
        let security_providers =
            self.security_providers.iter().map(|sp| sp.ident()).map(|ident| quote! { &#ident::OBJECT_LOOKUP });
        quote! {
            use root::lookup::CompositeObjectLookup;
            pub const OBJECT_LOOKUP: CompositeObjectLookup<0, #child_count> = CompositeObjectLookup{
                table_lookup: &root::core::all::table_id::TABLE_LOOKUP,
                children: [ #(#security_providers),* ],
            };
        }
    }

    fn generate(&self) -> TokenStream2 {
        let id = to_mod_identifier(&self.name);
        let security_providers = self.security_providers.iter().map(|x| x.generate());
        let lookup = self.generate_lookup();
        quote! {
            pub mod #id {
                use super::root;
                #(#security_providers)*
                #lookup
            }
        }
    }
}

impl SecurityProvider {
    fn parse(name: String, data: &Value) -> Result<Self, ParseError> {
        let tables = match data {
            Value::Object(tables) => {
                tables.iter().map(|(name, value)| Table::parse(name.clone(), value)).collect::<Result<Vec<_>, _>>()
            }
            _ => Err(ParseError::InvalidSecurityProvider(name.clone())),
        }?;
        Ok(Self { name, tables })
    }

    fn canonical_name(&self) -> &str {
        if self.name != "*" {
            &self.name
        } else {
            "all"
        }
    }

    fn ident(&self) -> Ident {
        to_mod_identifier(self.canonical_name())
    }

    fn generate_lookup(&self) -> TokenStream2 {
        let id = to_const_identifier(self.canonical_name());
        let sp = if self.name != "*" {
            quote! { self::super::admin::sp::#id.value() }
        } else {
            quote! { 0 + 0 } // This is silly, but otherwise there is a warning for unnecessary curly braces.
        };
        let child_count = self.tables.len();
        let tables = self.tables.iter().map(|table| table.ident()).map(|ident| quote! { &#ident::OBJECT_LOOKUP });
        quote! {
            use root::lookup::CompositeObjectLookup;
            pub const OBJECT_LOOKUP: CompositeObjectLookup<{#sp}, #child_count> = CompositeObjectLookup{
                table_lookup: &root::core::all::table_id::TABLE_LOOKUP,
                children: [ #(#tables),* ],
            };
        }
    }

    fn generate(&self) -> TokenStream2 {
        let id = self.ident();
        let tables = self.tables.iter().map(|x| x.generate());
        let lookup = self.generate_lookup();
        quote! {
            pub mod #id {
                use super::root;
                #(#tables)*
                #lookup
            }
        }
    }
}

impl Table {
    fn parse(name: String, data: &Value) -> Result<Self, ParseError> {
        let (uids, uid_ranges): (Vec<_>, Vec<_>) = match data {
            Value::Object(items) => Ok(items.iter().partition(|(name, _value)| !is_range(name.as_str()))),
            _ => Err(ParseError::InvalidTable(name.clone())),
        }?;
        let uids = uids
            .iter()
            .map(|(name, value)| UID::parse(name.to_string(), &value))
            .collect::<Result<Vec<_>, _>>()?;
        let uid_ranges = uid_ranges
            .iter()
            .map(|(name, value)| UIDRange::parse(name.to_string(), &value))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { name, uids, uid_ranges })
    }

    fn ident(&self) -> Ident {
        to_mod_identifier(&self.name)
    }

    fn generate_data(&self) -> TokenStream2 {
        let uids = self.uids.iter().map(|x| x.generate());
        let uid_ranges = self.uid_ranges.iter().map(|x| x.generate());
        quote! {
            #[allow(unused)]
            use root::UID;
            #[allow(unused)]
            use root::UIDRange;
            #[allow(unused)]
            use root::lookup::NameRange;

            #(#uids)*

            #(#uid_ranges)*
        }
    }

    fn generate_lookup(&self) -> TokenStream2 {
        let name = to_const_identifier(&self.name);

        let num_uids = self.uids.len();
        let num_uid_ranges = self.uid_ranges.len();

        let mut uids = self.uids.clone();
        let mut uid_ranges = self.uid_ranges.clone();

        uids.sort_by_key(|x| x.base);
        let uids_by_value: Vec<_> =
            uids.iter().map(|x| (x.ident(), x.name_ident())).map(|(x, y)| quote! {(#x, #y)}).collect();
        uids.sort_by_key(|x| x.name.clone());
        let uids_by_name: Vec<_> =
            uids.iter().map(|x| (x.name_ident(), x.ident())).map(|(x, y)| quote! {(#x, #y)}).collect();
        uid_ranges.sort_by_key(|x| x.base);
        let ranges_by_value: Vec<_> =
            uid_ranges.iter().map(|x| (x.ident(), x.name_ident())).map(|(x, y)| quote! {(#x, #y)}).collect();
        uid_ranges.sort_by_key(|x| x.prefix().to_string());
        let ranges_by_name: Vec<_> =
            uid_ranges.iter().map(|x| (x.name_ident(), x.ident())).map(|(x, y)| quote! {(#x, #y)}).collect();

        let table_lookup = if self.name == "TableID" {
            quote! {
                use root::lookup::ListTableLookup;
                pub const TABLE_LOOKUP: ListTableLookup<#num_uids> = ListTableLookup {
                    uids_by_name: [ #(#uids_by_name),* ]
                };
            }
        } else {
            quote! {}
        };

        let object_lookup = {
            let meta_tables = ["InvokingID", "General", "TableID"];
            let this_table = match !meta_tables.contains(&self.name.as_str()) {
                true => quote! { root::core::all::table_id::#name },
                false => quote! { UID::null() },
            };
            quote! {
                use root::lookup::ListObjectLookup;
                pub const THIS_TABLE: UID = #this_table;
                pub const OBJECT_LOOKUP: ListObjectLookup<{THIS_TABLE.value()}, #num_uids, #num_uid_ranges> = ListObjectLookup {
                    table_lookup: &root::core::all::table_id::TABLE_LOOKUP,
                    uids_by_value: [ #(#uids_by_value),* ],
                    uids_by_name: [ #(#uids_by_name),* ],
                    ranges_by_value: [ #(#ranges_by_value),* ],
                    ranges_by_name: [ #(#ranges_by_name),* ],
                };
            }
        };

        quote! {
            #object_lookup
            #table_lookup
        }
    }

    fn generate(&self) -> TokenStream2 {
        let id = self.ident();
        let data = self.generate_data();
        let lookup = self.generate_lookup();
        quote! {
            pub mod #id {
                use super::root;
                #data
                #lookup
            }
        }
    }
}

impl UID {
    fn parse(name: String, data: &Value) -> Result<Self, ParseError> {
        let base = match data {
            Value::Number(number) => number.as_u64().ok_or(ParseError::InvalidUIDBase(name.clone())),
            Value::String(s) => u64::from_str_radix(s, 16).map_err(|_| ParseError::InvalidUIDBase(name.clone())),
            _ => Err(ParseError::InvalidUIDFormat(name.clone())),
        }?;
        Ok(Self { name, base })
    }

    fn ident(&self) -> proc_macro2::Ident {
        to_const_identifier(&self.name)
    }

    fn name_ident(&self) -> proc_macro2::Ident {
        let id = self.ident();
        format_ident!("NAME_{id}")
    }

    fn generate(&self) -> TokenStream2 {
        let id = self.ident();
        let id_name = self.name_ident();
        let name = &self.name;
        let base = self.base;
        quote! {
            pub const #id: UID = UID::new(#base);
            const #id_name: &str = #name;
        }
    }
}

impl UIDRange {
    fn parse(name: String, data: &Value) -> Result<Self, ParseError> {
        let value_str = match data {
            Value::String(s) => Ok(s),
            _ => Err(ParseError::InvalidUIDFormat(name.clone())),
        }?;
        let value_parts: Vec<_> = value_str.split('-').collect();
        let base = value_parts
            .get(0)
            .ok_or(ParseError::InvalidUIDBase(name.clone()))
            .map(|n| u64::from_str_radix(&n, 16).map_err(|_| ParseError::InvalidUIDBase(name.clone())))??;
        let count = value_parts
            .get(1)
            .ok_or(ParseError::InvalidUIDCount(name.clone()))
            .map(|n| n.parse::<u64>().map_err(|_| ParseError::InvalidUIDCount(name.clone())))??;
        let step = value_parts
            .get(2)
            .map(|n| n.parse::<u64>().map_err(|_| ParseError::InvalidUIDStep(name.clone())))
            .unwrap_or(Ok(1))?;
        Ok(Self { name, base, count, step })
    }

    fn prefix(&self) -> &str {
        match self.name.find('{') {
            Some(idx) => &self.name[..idx],
            None => &self.name,
        }
    }

    fn suffix(&self) -> &str {
        match self.name.find('}') {
            Some(idx) => &self.name[(idx + 1)..],
            None => "",
        }
    }

    fn ident(&self) -> proc_macro2::Ident {
        to_const_identifier(&format!("{}{}", self.prefix(), self.suffix()))
    }

    fn name_ident(&self) -> proc_macro2::Ident {
        let id = self.ident();
        format_ident!("NAME_{id}")
    }

    fn generate(&self) -> TokenStream2 {
        let id = self.ident();
        let id_name = self.name_ident();
        let prefix = self.prefix();
        let suffix = self.suffix();
        let name = quote! { NameRange{ prefix: #prefix, suffix: #suffix }};
        let base = self.base;
        let count = self.count;
        let step = self.step;
        quote! {
            pub const #id: UIDRange = UIDRange::new_count(UID::new(#base), #count, #step);
            const #id_name: NameRange = #name;
        }
    }
}

fn parse(spec_json: &Value) -> Result<Vec<Feature>, ParseError> {
    match spec_json {
        Value::Object(features) => features
            .iter()
            .map(|(name, value)| Feature::parse(name.clone(), value))
            .collect::<Result<Vec<_>, _>>(),
        _ => Err(ParseError::InvalidFile),
    }
}

fn generate(features: &[Feature]) -> TokenStream2 {
    let features = features.iter().map(|x| x.generate());
    quote! {
        #(#features)*
    }
}

fn main() -> Result<(), ()> {
    let spec_path = "src/spec/spec.json";
    let spec_file = File::open(spec_path).unwrap();
    let spec_json: Value = serde_json::from_reader(spec_file).unwrap();
    let spec_data = parse(&spec_json).unwrap();

    let content = generate(&spec_data);

    let mut out_path = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    out_path.push("spec.rs");
    let mut out_file = File::create(out_path).unwrap();
    out_file.write_all(content.to_string().as_bytes()).unwrap();
    println!("cargo::rerun-if-changed={spec_path}");
    Ok(())
}
