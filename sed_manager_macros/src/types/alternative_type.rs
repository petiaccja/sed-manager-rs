use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, TokenStreamExt};
use syn::{self, parse_macro_input, DeriveInput};

use crate::parse::data_enum::DataEnum;

fn gen_uid_patterns(declaration: &DataEnum) -> TokenStream2 {
    let name = &declaration.name;
    let mut uid_patterns = quote! {};
    for variant in &declaration.variants {
        let variant_name = &variant.name;
        let variant_type = &variant.ty;
        let pattern = quote! { #name::#variant_name(value) => <#variant_type as ::sed_manager::specification::basic_types::Type>::uid(), };
        uid_patterns.append_all(pattern);
    }
    uid_patterns
}

fn gen_value_patterns(declaration: &DataEnum) -> TokenStream2 {
    let name = &declaration.name;
    let mut value_patterns = quote! {};
    for variant in &declaration.variants {
        let variant_name = &variant.name;
        let pattern = quote! { #name::#variant_name(value) => ::sed_manager::messaging::value::Value::from(value), };
        value_patterns.append_all(pattern);
    }
    value_patterns
}

fn gen_parse_patterns(declaration: &DataEnum) -> TokenStream2 {
    let name = &declaration.name;
    let mut parse_patterns = quote! {};
    for variant in &declaration.variants {
        let variant_name = &variant.name;
        let variant_type = &variant.ty;
        let pattern = quote! {
            x if x == <#variant_type as ::sed_manager::specification::basic_types::Type>::uid() => {
                match #variant_type::try_from(named.value) {
                    Ok(alt) => Ok(#name::#variant_name(alt)),
                    Err(named_val) => Err(::sed_manager::messaging::value::Value::from(
                        ::sed_manager::messaging::value::Named{ name: named.name, value: named_val }
                    ))
                }
            }
        };
        parse_patterns.append_all(pattern);
    }
    parse_patterns
}

fn gen_to_value(declaration: &DataEnum) -> TokenStream2 {
    let name = &declaration.name;
    let uid_patterns = gen_uid_patterns(declaration);
    let value_patterns = gen_value_patterns(declaration);
    quote! {
        impl ::core::convert::From<#name> for ::sed_manager::messaging::value::Value {
            fn from(value: #name) -> Self {
                let uid = match &value {
                    #uid_patterns
                };
                let name = ::sed_manager::messaging::value::Value::from(::std::vec::Vec::from(&uid.value().to_be_bytes()[4..]));
                let value = match value {
                    #value_patterns
                };
                ::sed_manager::messaging::value::Value::from(::sed_manager::messaging::value::Named{
                    name,
                    value,
                })
            }
        }
    }
}

fn gen_try_from_alt(declaration: &DataEnum) -> TokenStream2 {
    let name = &declaration.name;
    let parse_patterns = gen_parse_patterns(declaration);
    quote! {
        impl ::core::convert::TryFrom<::sed_manager::messaging::value::Value> for #name {
            type Error = ::sed_manager::messaging::value::Value;
            fn try_from(value: ::sed_manager::messaging::value::Value) -> Result<Self, Self::Error> {
                let named = ::sed_manager::messaging::value::Named::try_from(value)?;
                let Ok(bytes) = &::sed_manager::messaging::value::Bytes::try_from(&named.name) else {
                    return Err(::sed_manager::messaging::value::Value::from(named));
                };
                if bytes.len() == 4 {
                    let mut extended = [0u8; 8];
                    for i in 0..4 {
                        extended[4 + i] = bytes[i];
                    }
                    let uid = ::sed_manager::messaging::uid::UID::new(u64::from_be_bytes(extended) | 0x0000_0005_0000_0000);
                    match uid {
                        #parse_patterns
                        _ => Err(::sed_manager::messaging::value::Value::from(named)),
                    }
                }
                else {
                    Err(::sed_manager::messaging::value::Value::from(named))
                }
            }
        }
    }
}

pub fn derive_alternative_type(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as DeriveInput);
    let declaration = match DataEnum::parse(&input) {
        Ok(declaration) => declaration,
        Err(err) => return err.to_compile_error().into(),
    };
    let mut impls = quote! {};
    impls.append_all(gen_to_value(&declaration));
    impls.append_all(gen_try_from_alt(&declaration));
    impls.into()
}
