use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, TokenStreamExt};
use syn::{self, parse_macro_input, DeriveInput};

use crate::parse::alias_struct::AliasStruct;

fn gen_to_value(alias_struct: &AliasStruct) -> TokenStream2 {
    let name = &alias_struct.name;
    quote! {
        impl ::core::convert::From<#name> for ::sed_manager::messaging::value::Value {
            fn from(value: #name) -> Self {
                ::sed_manager::messaging::value::Value::from(value.0)
            }
        }
    }
}

fn gen_try_from_alias(alias_struct: &AliasStruct) -> TokenStream2 {
    let name = &alias_struct.name;
    let ty = &alias_struct.ty;
    quote! {
        impl ::core::convert::TryFrom<::sed_manager::messaging::value::Value> for #name {
            type Error = ::sed_manager::messaging::value::Value;
            fn try_from(value: ::sed_manager::messaging::value::Value) -> Result<Self, Self::Error> {
                Ok(Self(#ty::try_from(value)?))
            }
        }
    }
}

fn gen_deref(alias_struct: &AliasStruct) -> TokenStream2 {
    let name = &alias_struct.name;
    let ty = &alias_struct.ty;
    quote! {
        impl ::std::ops::Deref for #name {
            type Target = #ty;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    }
}

fn gen_deref_mut(alias_struct: &AliasStruct) -> TokenStream2 {
    let name = &alias_struct.name;
    quote! {
        impl ::std::ops::DerefMut for #name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    }
}

pub fn derive_alias_type(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as DeriveInput);
    let alias_struct = match AliasStruct::parse(&input) {
        Ok(declaration) => declaration,
        Err(err) => return err.to_compile_error().into(),
    };
    let mut impls = quote! {};
    impls.append_all(gen_to_value(&alias_struct));
    impls.append_all(gen_try_from_alias(&alias_struct));
    impls.append_all(gen_deref(&alias_struct));
    impls.append_all(gen_deref_mut(&alias_struct));
    impls.into()
}
