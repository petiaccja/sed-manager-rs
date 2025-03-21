//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, TokenStreamExt};
use syn::{self, parse_macro_input, DeriveInput};

use crate::parse::data_struct::DataStruct;

fn gen_to_tuple(data_struct: &DataStruct, struct_instance: &syn::Ident) -> TokenStream2 {
    let mut elements = quote! {};
    for field in &data_struct.fields {
        let field_name = &field.name;
        elements.append_all(quote! { #struct_instance.#field_name, });
    }
    quote! {(#elements)}
}

fn gen_from_tuple(data_struct: &DataStruct, tuple_instance: &syn::Ident) -> TokenStream2 {
    let name = &data_struct.name;
    let mut elements = quote! {};
    for (idx, field) in data_struct.fields.iter().enumerate() {
        let field_name = &field.name;
        let tuple_field = syn::Index::from(idx);
        elements.append_all(quote! { #field_name: #tuple_instance.#tuple_field, });
    }
    quote! { #name{ #elements }}
}

fn gen_tuple_type(data_struct: &DataStruct) -> TokenStream2 {
    let mut elements = quote! {};
    for field in &data_struct.fields {
        let field_ty = &field.ty;
        elements.append_all(quote! { #field_ty, });
    }
    quote! { ( #elements )}
}

fn gen_to_value(data_struct: &DataStruct) -> TokenStream2 {
    let name = &data_struct.name;
    let to_tuple = gen_to_tuple(data_struct, &syn::Ident::new("value", Span::call_site()));
    quote! {
        impl ::core::convert::From<#name> for ::sed_manager::messaging::value::Value {
            fn from(value: #name) -> Self {
                let tuple = #to_tuple;
                let args = ::sed_manager::rpc::args::IntoMethodArgs::into_method_args(tuple);
                ::sed_manager::messaging::value::Value::from(args)
            }
        }
    }
}

fn gen_try_from_alias(data_struct: &DataStruct) -> TokenStream2 {
    let name = &data_struct.name;
    let tuple_ty = gen_tuple_type(data_struct);
    let from_tuple = gen_from_tuple(data_struct, &syn::Ident::new("tuple", Span::call_site()));
    quote! {
        impl ::core::convert::TryFrom<::sed_manager::messaging::value::Value> for #name {
            type Error = ::sed_manager::messaging::value::Value;
            fn try_from(value: ::sed_manager::messaging::value::Value) -> Result<Self, Self::Error> {
                let args = ::sed_manager::messaging::value::List::try_from(value)?;
                let maybe_tuple = <#tuple_ty as ::sed_manager::rpc::args::TryFromMethodArgs>::try_from_method_args(args.clone());
                match maybe_tuple {
                    ::core::result::Result::Ok(tuple) => ::core::result::Result::Ok(#from_tuple),
                    ::core::result::Result::Err(_) => ::core::result::Result::Err(::sed_manager::messaging::value::Value::from(args)),
                }
            }
        }
    }
}

pub fn derive_struct_type(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as DeriveInput);
    let alias_struct = match DataStruct::parse(&input) {
        Ok(declaration) => declaration,
        Err(err) => return err.to_compile_error().into(),
    };
    let mut impls = quote! {};
    impls.append_all(gen_to_value(&alias_struct));
    impls.append_all(gen_try_from_alias(&alias_struct));
    impls.into()
}
