use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, TokenStreamExt};

use crate::parse::numeric_enum::NumericEnum;

pub fn gen_serialize_enum(numeric_enum: &NumericEnum) -> TokenStream2 {
    let name = &numeric_enum.name;
    let repr = &numeric_enum.repr;
    let mut variants = TokenStream2::new();
    for variant in &numeric_enum.variants {
        let ident = format_ident!("{}", variant.name);
        let pattern = quote! { #name::#ident => #name::#ident as #repr, };
        variants.append_all(pattern);
    }
    quote! {
        impl ::sed_manager::serialization::Serialize<u8> for #name {
            type Error = ::sed_manager::serialization::Error;
            fn serialize(&self, stream: &mut ::sed_manager::serialization::OutputStream<u8>) -> ::core::result::Result<(), Self::Error> {
                let discr = match self {
                    #variants
                };
                match discr.serialize(stream) {
                    ::core::result::Result::Ok(_) => ::core::result::Result::Ok(()),
                    ::core::result::Result::Err(err) => ::core::result::Result::Err(err.into()),
                }
            }
        }
    }
}

pub fn gen_deserialize_enum(enum_desc: &NumericEnum) -> TokenStream2 {
    let name = &enum_desc.name;
    let repr = &enum_desc.repr;
    let mut variants = TokenStream2::new();
    for variant in &enum_desc.variants {
        let ident = format_ident!("{}", variant.name);
        let pattern = quote! { x if x == (#name::#ident as #repr) => ::core::result::Result::Ok(#name::#ident), };
        variants.append_all(pattern);
    }
    let catch_all = match &enum_desc.fallback {
        Some(variant) => quote! { ::core::result::Result::Ok(#name::#variant) },
        None => quote! { ::core::result::Result::Err(::sed_manager::serialization::Error::InvalidData) },
    };
    quote! {
        impl ::sed_manager::serialization::Deserialize<u8> for #name {
            type Error = ::sed_manager::serialization::Error;
            fn deserialize(stream: &mut ::sed_manager::serialization::InputStream<u8>) -> ::core::result::Result<Self, Self::Error> {
                let discr = match #repr::deserialize(stream) {
                    ::core::result::Result::Ok(discr) => discr,
                    ::core::result::Result::Err(err) => return ::core::result::Result::Err(err.into()),
                };
                match discr {
                    #variants
                    _ => #catch_all,
                }
            }
        }
    }
}
