//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::{collections::HashMap, str::FromStr as _};

use proc_macro2;
use quote::{format_ident, quote};
use serde::{Deserialize, Deserializer, de::Error as _};
use syn::{File, Ident, ItemConst, ItemMod, parse_quote};

type Error = Box<dyn std::error::Error>;

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
    },
}

impl Object {
    pub fn generate(&self, name: &str) -> Result<ItemConst, Error> {
        let name = name.replace("{n}", "").replace("__", "_");
        let const_ident = to_const_ident(&name);
        match self {
            Object::Unique(base) => {
                let base_hex = proc_macro2::Literal::from_str(&format!("0x{base:016x}_u64")).unwrap();
                Ok(parse_quote! {
                    pub const #const_ident : ObjectRef<{THIS_TABLE.to_u64()}> = ObjectRef::new_unchecked(#base_hex);
                })
            }
            Object::Range { base, count, step } => {
                let base_hex = proc_macro2::Literal::from_str(&format!("0x{base:016x}_u64")).unwrap();
                Ok(parse_quote! {
                    pub const #const_ident : ObjectRange<{THIS_TABLE.to_u64()}> = ObjectRange {
                        start: ObjectRef::new_unchecked(#base_hex),
                        end: ObjectRef::new_unchecked(#base_hex + #count * #step),
                        step: #step as u32,
                    };
                })
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct Table(HashMap<String, Object>);

impl Table {
    pub fn generate(&self, name: &str) -> Result<ItemMod, Error> {
        let table_name = to_mod_ident(name);
        if name == "TableID" {
            let ids = self.0.iter().map(|(name, object)| {
                if let Object::Unique(base) = object {
                    let object_name = to_const_ident(name);
                    let base_hex = proc_macro2::Literal::from_str(&format!("0x{base:016x}_u64")).unwrap();
                    quote! { pub const #object_name : TableRef = TableRef::new_unchecked(#base_hex); }
                } else {
                    quote! {}
                }
            });
            Ok(parse_quote! {
                pub mod #table_name {
                    use ::sed_packet::{TableRef};

                    #(#ids)*
                }
            })
        } else if name == "General" || name == "InvokingID" || name == "SMMethodID" {
            let ids = self.0.iter().map(|(name, object)| {
                if let Object::Unique(base) = object {
                    let object_name = to_const_ident(name);
                    let base_hex = proc_macro2::Literal::from_str(&format!("0x{base:016x}_u64")).unwrap();
                    quote! { pub const #object_name : Uid = Uid::new(#base_hex); }
                } else {
                    quote! {}
                }
            });
            Ok(parse_quote! {
                pub mod #table_name {
                    use ::sed_packet::{Uid};

                    #(#ids)*
                }
            })
        } else {
            let table_ref = to_const_ident(name);
            let objects =
                self.0.iter().map(|(name, object)| object.generate(name)).collect::<Result<Vec<ItemConst>, _>>()?;

            Ok(parse_quote! {
                pub mod #table_name {
                    use ::sed_packet::{TableRef, ObjectRef, ObjectRange};

                    const THIS_TABLE : TableRef = super::super::super::core::shared::table_id::#table_ref;

                    #(#objects)*
                }
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct SecurityProvider(HashMap<String, Table>);

impl SecurityProvider {
    pub fn generate(&self) -> Result<File, Error> {
        let tables = self.0.iter().map(|(name, table)| table.generate(name)).collect::<Result<Vec<ItemMod>, _>>()?;
        Ok(parse_quote!(
            #(#tables)*
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct Feature(HashMap<String, SecurityProvider>);

impl Feature {
    pub fn generate(&self) -> Result<File, Error> {
        let security_providers = self
            .0
            .iter()
            .map(|(name, security_provider)| {
                let name = to_mod_ident(if name == "*" { "shared" } else { name });
                let feature = security_provider.generate()?;
                Result::<ItemMod, Error>::Ok(parse_quote! {
                    pub mod #name {
                        #feature
                    }
                })
            })
            .collect::<Result<Vec<ItemMod>, _>>()?;
        Ok(parse_quote!(
            #(#security_providers)*
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct Spec(HashMap<String, Feature>);

impl Spec {
    pub fn generate(&self) -> Result<File, Error> {
        let features = self
            .0
            .iter()
            .map(|(name, feature)| {
                let name = to_mod_ident(name);
                let feature = feature.generate()?;
                Result::<ItemMod, Error>::Ok(parse_quote! {
                    pub mod #name {
                        #feature
                    }
                })
            })
            .collect::<Result<Vec<ItemMod>, _>>()?;
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
            "*": {
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
                        "base": "0000000900000200",
                        "count": 256,
                        "step": 1
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
                    "*".into(),
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
                            ("Admin{n}".into(), Object::Range { base: 0x0000000900000200, count: 256, step: 1 }),
                        ])),
                    )])),
                )])),
            ),
        ]));
        assert_eq!(serde_json::from_str::<Spec>(TEST_SPEC).unwrap(), expected);
    }

    #[test]
    fn generate_example() {
        let spec = generate_spec(TEST_SPEC).unwrap();
        println!("{spec}")
    }
}
