extern crate proc_macro;

use proc_macro::TokenStream;
use syn::{parse_macro_input, spanned::Spanned, DeriveInput};

mod gen;
mod parse;
mod validate;

use gen::{gen_deserialize_enum, gen_deserialize_struct, gen_serialize_enum, gen_serialize_struct};
use parse::{parse_enum, parse_struct};
use validate::{validate_enum, validate_struct};

pub fn derive_serialize(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as DeriveInput);
    match input.data {
        syn::Data::Struct(_) => {
            let desc = match parse_struct(&input) {
                Ok(struct_desc) => struct_desc,
                Err(err) => return err.to_compile_error().into(),
            };
            if let Err(err) = validate_struct(&desc) {
                return syn::Error::new(input.span(), format!("{}", err)).to_compile_error().into();
            }
            gen_serialize_struct(&desc).into()
        }
        syn::Data::Enum(_) => {
            let desc = match parse_enum(&input) {
                Ok(struct_desc) => struct_desc,
                Err(err) => return err.to_compile_error().into(),
            };
            if let Err(err) = validate_enum(&desc) {
                return syn::Error::new(input.span(), format!("{}", err)).to_compile_error().into();
            }
            gen_serialize_enum(&desc).into()
        }
        _ => syn::Error::new(input.span(), "only structs and enums are supported").to_compile_error().into(),
    }
}

pub fn derive_deserialize(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as DeriveInput);
    match input.data {
        syn::Data::Struct(_) => {
            let struct_desc = match parse_struct(&input) {
                Ok(struct_desc) => struct_desc,
                Err(err) => return err.to_compile_error().into(),
            };
            if let Err(err) = validate_struct(&struct_desc) {
                return syn::Error::new(input.span(), format!("{}", err)).to_compile_error().into();
            }
            gen_deserialize_struct(&struct_desc).into()
        }
        syn::Data::Enum(_) => {
            let desc = match parse_enum(&input) {
                Ok(struct_desc) => struct_desc,
                Err(err) => return err.to_compile_error().into(),
            };
            gen_deserialize_enum(&desc).into()
        }
        _ => syn::Error::new(input.span(), "only structs and enums are supported").to_compile_error().into(),
    }
}
