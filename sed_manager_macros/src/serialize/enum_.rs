use std::{collections::HashMap, sync::LazyLock};

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, TokenStreamExt};

use crate::parse::numeric_enum::NumericEnum;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ConversionKind {
    None,
    Always,
    Try,
}

static CONVERSION_TABLE: LazyLock<HashMap<&str, ConversionKind>> = LazyLock::new(|| {
    [
        // S -> S
        ("i8->i8", ConversionKind::None),
        ("i8->i16", ConversionKind::Always),
        ("i8->i32", ConversionKind::Always),
        ("i8->i64", ConversionKind::Always),
        ("i16->i8", ConversionKind::Try),
        ("i16->i16", ConversionKind::None),
        ("i16->i32", ConversionKind::Always),
        ("i16->i64", ConversionKind::Always),
        ("i32->i8", ConversionKind::Try),
        ("i32->i16", ConversionKind::Try),
        ("i32->i32", ConversionKind::None),
        ("i32->i64", ConversionKind::Always),
        ("i64->i8", ConversionKind::Try),
        ("i64->i16", ConversionKind::Try),
        ("i64->i32", ConversionKind::Try),
        ("i64->i64", ConversionKind::None),
        // U -> U
        ("u8->u8", ConversionKind::None),
        ("u8->u16", ConversionKind::Always),
        ("u8->u32", ConversionKind::Always),
        ("u8->u64", ConversionKind::Always),
        ("u16->u8", ConversionKind::Try),
        ("u16->u16", ConversionKind::None),
        ("u16->u32", ConversionKind::Always),
        ("u16->u64", ConversionKind::Always),
        ("u32->u8", ConversionKind::Try),
        ("u32->u16", ConversionKind::Try),
        ("u32->u32", ConversionKind::None),
        ("u32->u64", ConversionKind::Always),
        ("u64->u8", ConversionKind::Try),
        ("u64->u16", ConversionKind::Try),
        ("u64->u32", ConversionKind::Try),
        ("u64->u64", ConversionKind::None),
        //  S -> U
        ("i8->u8", ConversionKind::Try),
        ("i8->u16", ConversionKind::Try),
        ("i8->u32", ConversionKind::Try),
        ("i8->u64", ConversionKind::Try),
        ("i16->u8", ConversionKind::Try),
        ("i16->u16", ConversionKind::Try),
        ("i16->u32", ConversionKind::Try),
        ("i16->u64", ConversionKind::Try),
        ("i32->u8", ConversionKind::Try),
        ("i32->u16", ConversionKind::Try),
        ("i32->u32", ConversionKind::Try),
        ("i32->u64", ConversionKind::Try),
        ("i64->u8", ConversionKind::Try),
        ("i64->u16", ConversionKind::Try),
        ("i64->u32", ConversionKind::Try),
        ("i64->u64", ConversionKind::Try),
        //  U -> S
        ("u8->i8", ConversionKind::Try),
        ("u8->i16", ConversionKind::Try),
        ("u8->i32", ConversionKind::Try),
        ("u8->i64", ConversionKind::Try),
        ("u16->i8", ConversionKind::Try),
        ("u16->i16", ConversionKind::Try),
        ("u16->i32", ConversionKind::Try),
        ("u16->i64", ConversionKind::Try),
        ("u32->i8", ConversionKind::Try),
        ("u32->i16", ConversionKind::Try),
        ("u32->i32", ConversionKind::Try),
        ("u32->i64", ConversionKind::Try),
        ("u64->i8", ConversionKind::Try),
        ("u64->i16", ConversionKind::Try),
        ("u64->i32", ConversionKind::Try),
        ("u64->i64", ConversionKind::Try),
    ]
    .into_iter()
    .collect()
});

fn get_conversion_kind(source_ty: &str, target_ty: &str) -> ConversionKind {
    let conv = format!("{source_ty}->{target_ty}");
    match CONVERSION_TABLE.get(conv.as_str()) {
        Some(conv_kind) => *conv_kind,
        None => ConversionKind::None,
    }
}

fn gen_into_repr(numeric_enum: &NumericEnum) -> TokenStream2 {
    let name = &numeric_enum.name;
    let repr = &numeric_enum.repr;
    let mut variant_match_cases = TokenStream2::new();
    for variant in &numeric_enum.variants {
        let ident = format_ident!("{}", variant.name);
        let pattern = quote! { #name::#ident => #name::#ident as #repr, };
        variant_match_cases.append_all(pattern);
    }
    quote! {
        impl ::core::convert::From<&#name> for #repr {
            fn from(value: &#name) -> Self {
                match value {
                    #variant_match_cases
                }
            }
        }

        impl ::core::convert::From<#name> for #repr {
            fn from(value: #name) -> Self {
                #repr::from(&value)
            }
        }
    }
}

fn gen_try_from_repr(numeric_enum: &NumericEnum) -> TokenStream2 {
    let name = &numeric_enum.name;
    let repr = &numeric_enum.repr;
    let mut variant_match_cases = TokenStream2::new();
    for variant in &numeric_enum.variants {
        let ident = format_ident!("{}", variant.name);
        let pattern = quote! { x if x == (#name::#ident as #repr) => ::core::result::Result::Ok(#name::#ident), };
        variant_match_cases.append_all(pattern);
    }
    let catch_all = match &numeric_enum.fallback {
        Some(variant) => quote! { ::core::result::Result::Ok(#name::#variant) },
        None => quote! { ::core::result::Result::Err(value) },
    };
    quote! {
        impl ::core::convert::TryFrom<#repr> for #name {
            type Error = #repr;
            fn try_from(value: #repr) -> ::core::result::Result<Self, Self::Error> {
                match value {
                    #variant_match_cases
                    _ => #catch_all,
                }
            }
        }
    }
}

fn gen_into_or_try_into_integer(numeric_enum: &NumericEnum) -> TokenStream2 {
    let name = &numeric_enum.name;
    let repr = &numeric_enum.repr;
    let target_tys = ["i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64"];
    let mut conversion_impls = quote! {};
    for target_ty in target_tys {
        let target_id = format_ident!("{}", target_ty);
        let kind = get_conversion_kind(&repr.to_string(), target_ty);
        let converion_impl = if kind == ConversionKind::Always {
            quote! {
                impl ::core::convert::From<#name> for #target_id {
                    fn from(value: #name) -> #target_id {
                        #target_id::from(#repr::from(value))
                    }
                }
            }
        } else if kind == ConversionKind::Try {
            quote! {
                impl ::core::convert::TryFrom<#name> for #target_id {
                    type Error = <#target_id as ::core::convert::TryFrom<#repr>>::Error;
                    fn try_from(value: #name) -> ::core::result::Result<Self, Self::Error> {
                        #target_id::try_from(#repr::from(value))
                    }
                }
            }
        } else {
            quote! {}
        };
        conversion_impls.append_all(converion_impl);
    }
    conversion_impls
}

fn gen_from_or_try_from_integer(numeric_enum: &NumericEnum) -> TokenStream2 {
    let name = &numeric_enum.name;
    let repr = &numeric_enum.repr;
    let source_tys = ["i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64"];
    let mut conversion_impls = quote! {};
    for source_ty in source_tys {
        let source_id = format_ident!("{}", source_ty);
        let kind = get_conversion_kind(source_ty, &repr.to_string());
        let converion_impl = if kind != ConversionKind::None {
            quote! {
                impl ::core::convert::TryFrom<#source_id> for #name {
                    type Error = <#name as ::core::convert::TryFrom<#repr>>::Error;
                    fn try_from(value: #source_id) -> ::core::result::Result<Self, Self::Error> {
                        #name::try_from(#repr::try_from(value).map_err(|_| value as Self::Error)?)
                    }
                }
            }
        } else {
            quote! {}
        };
        conversion_impls.append_all(converion_impl);
    }
    conversion_impls
}

pub fn gen_serialize_enum(numeric_enum: &NumericEnum) -> TokenStream2 {
    let name = &numeric_enum.name;
    let repr = &numeric_enum.repr;
    let into_repr = gen_into_repr(numeric_enum);
    let conversion_impls = gen_into_or_try_into_integer(numeric_enum);

    quote! {
        #into_repr
        #conversion_impls

        impl ::sed_manager::serialization::Serialize<u8> for #name {
            type Error = ::sed_manager::serialization::Error;
            fn serialize(&self, stream: &mut ::sed_manager::serialization::OutputStream<u8>) -> ::core::result::Result<(), Self::Error> {
                let discr = #repr :: from(self);
                discr.serialize(stream).map_err(|err| Self::Error::from(err))
            }
        }
    }
}

pub fn gen_deserialize_enum(numeric_enum: &NumericEnum) -> TokenStream2 {
    let name = &numeric_enum.name;
    let repr = &numeric_enum.repr;
    let try_from_repr = gen_try_from_repr(numeric_enum);
    let conversion_impls = gen_from_or_try_from_integer(numeric_enum);

    quote! {
        #try_from_repr
        #conversion_impls

        impl ::sed_manager::serialization::Deserialize<u8> for #name {
            type Error = ::sed_manager::serialization::Error;
            fn deserialize(stream: &mut ::sed_manager::serialization::InputStream<u8>) -> ::core::result::Result<Self, Self::Error> {
                let discr = #repr::deserialize(stream).map_err(|err| Self::Error::from(err))?;
                #name::try_from(discr).map_err(|_| ::sed_manager::serialization::Error::InvalidData)
            }
        }
    }
}
