extern crate proc_macro;

use proc_macro::TokenStream;
use syn::{parse_macro_input, spanned::Spanned, DeriveInput};

mod gen;
mod parse;
mod validate;

use gen::{gen_deserialize_struct, gen_serialize_struct};
use parse::parse_struct;
use validate::validate_struct;

#[proc_macro_derive(Serialize, attributes(layout))]
pub fn derive_serialize(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as DeriveInput);
    let struct_desc = match parse_struct(&input) {
        Ok(struct_desc) => struct_desc,
        Err(err) => return err.to_compile_error().into(),
    };
    if let Err(err) = validate_struct(&struct_desc) {
        return syn::Error::new(input.span(), format!("{}", err)).to_compile_error().into();
    }
    gen_serialize_struct(&struct_desc).into()
}

#[proc_macro_derive(Deserialize)]
pub fn derive_deserialize(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as DeriveInput);
    let struct_desc = match parse_struct(&input) {
        Ok(struct_desc) => struct_desc,
        Err(err) => return err.to_compile_error().into(),
    };
    if let Err(err) = validate_struct(&struct_desc) {
        return syn::Error::new(input.span(), format!("{}", err)).to_compile_error().into();
    }
    gen_deserialize_struct(&struct_desc).into()
}
