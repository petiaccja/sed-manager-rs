extern crate proc_macro;
use core::fmt;
use std::fmt::Debug;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, format_ident};
use syn::{self, parse_macro_input, spanned::Spanned, DataStruct, DeriveInput};

#[derive(Debug)]
struct FieldLayout {
    pub offset: Option<usize>,
}

struct Field {
    pub name: String,
    pub layout: Option<FieldLayout>,
}

fn parse_field_layout(attr: syn::Attribute) -> Result<FieldLayout, syn::Error> {
    let mut layout = FieldLayout { offset: None };
    let valid = attr.parse_nested_meta(|param| {
        let mut offset: Option<usize> = None;
        if param.path.is_ident("offset") {
            let value = param.value()?;
            let int_value: syn::LitInt = value.parse()?;
            offset = Some(int_value.base10_parse::<usize>()?);
        } else {
            return Err(param.error("invalid layout parameter"));
        }
        layout = FieldLayout { offset: offset };
        return Ok(());
    });
    match valid {
        Ok(_) => Ok(layout),
        Err(err) => Err(err),
    }
}

fn parse_struct_layout(ast: DataStruct) -> Result<Vec<Field>, syn::Error> {
    let mut fields = Vec::<Field>::new();
    for field in ast.fields {
        if let Some(ident) = field.ident {
            let mut layout = None;
            for attr in field.attrs {
                if attr.path().is_ident("layout") {
                    layout = parse_field_layout(attr)?.into();
                }
            }
            fields.push(Field { name: ident.to_string(), layout: layout });
        } else {
            return Err(syn::Error::new(field.span(), "field must have a name"));
        }
    }
    Ok(vec![])
}

fn derive_serialize_from_fields(name: &str, fields: &[Field]) -> TokenStream2 {
    let name_ident = format_ident!("{}", name);
    let tokens_fields = quote! {
        print!(#name);
    };
    quote! {
        impl ::sed_manager::serialization::Serialize<#name_ident> for #name_ident {
            fn serialize(&self) -> ::std::vec::Vec<u8> {
                #tokens_fields
            }
        }
    }
}

fn derive_serialize_from_data(name: &str, data: DataStruct) -> TokenStream2 {
    match parse_struct_layout(data) {
        Ok(fields) => derive_serialize_from_fields(name, fields.as_slice()),
        Err(err) => err.to_compile_error(),
    }
}

fn derive_serialize_impl(derive_input: DeriveInput) -> TokenStream2 {
    let name = derive_input.ident.to_string();
    match derive_input.data {
        syn::Data::Struct(struct_data) => derive_serialize_from_data(name.as_str(), struct_data),
        _ => syn::Error::new(derive_input.ident.span(), "can only derive Serialize for structs").to_compile_error(),
    }
}

#[proc_macro_derive(Serialize, attributes(layout))]
pub fn derive_serialize(tokens: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(tokens as DeriveInput);
    derive_serialize_impl(derive_input).into()
}

#[proc_macro_derive(Deserialize)]
pub fn derive_deserialize(_: TokenStream) -> TokenStream {
    TokenStream::new()
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::*;

    #[test]
    fn it_works() {
        let stream = quote! {
            struct Data {
                pub field_a : u32,
                #[layout(offset=32)]
                pub field_b : u16,
            }
        };
        let input = syn::parse2::<DeriveInput>(stream).unwrap();
        let tokens = derive_serialize_impl(input);
        println!("{}", tokens);
    }
}
