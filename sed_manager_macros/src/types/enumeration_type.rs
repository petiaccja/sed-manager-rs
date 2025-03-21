//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, TokenStreamExt};
use syn::{parse_macro_input, DeriveInput};

use crate::parse::numeric_enum::NumericEnum;

fn gen_to_value(decl: &NumericEnum) -> TokenStream2 {
    let name = &decl.name;
    let repr = &decl.repr;
    quote! {
        impl ::core::convert::From<#name> for ::sed_manager::messaging::value::Value {
            fn from(value: #name) -> Self {
                Self::from(value as #repr)
            }
        }
    }
}

fn gen_try_from_patterns(desc: &NumericEnum) -> TokenStream2 {
    let name = &desc.name;
    let repr = &desc.repr;
    let mut patterns = quote! {};
    for variant in &desc.variants {
        let variant_name = format_ident!("{}", &variant.name);
        let pattern =
            quote! { x if x == #name::#variant_name as #repr => ::core::result::Result::Ok(#name::#variant_name), };
        patterns.append_all(pattern);
    }
    let catch_all = if let Some(fallback) = &desc.fallback {
        quote! { _ => ::core::result::Result::Ok(#name::#fallback), }
    } else {
        quote! { _ => ::core::result::Result::Err(::sed_manager::messaging::value::Value::from(repr)), }
    };
    patterns.append_all(catch_all);
    patterns
}

fn gen_try_from_enum(desc: &NumericEnum) -> TokenStream2 {
    let name = &desc.name;
    let repr = &desc.repr;
    let patterns = gen_try_from_patterns(desc);
    quote! {
        impl ::core::convert::TryFrom<::sed_manager::messaging::value::Value> for #name {
            type Error = ::sed_manager::messaging::value::Value;
            fn try_from(value: ::sed_manager::messaging::value::Value) -> ::core::result::Result<Self, Self::Error> {
                let repr = #repr::try_from(value)?;
                match repr {
                    #patterns
                }
            }
        }
    }
}

pub fn derive_enumeration_type(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as DeriveInput);
    let desc = match NumericEnum::parse(&input) {
        Ok(declaration) => declaration,
        Err(err) => return err.to_compile_error().into(),
    };
    let mut impls = quote! {};
    impls.append_all(gen_to_value(&desc));
    impls.append_all(gen_try_from_enum(&desc));
    impls.into()
}
