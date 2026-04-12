mod method;
mod object;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn object(attribute: TokenStream, item: TokenStream) -> TokenStream {
    let item = match syn::parse(item) {
        Ok(item) => item,
        Err(err) => return err.into_compile_error().into(),
    };
    let attribute = match syn::parse(attribute) {
        Ok(item) => item,
        Err(err) => return err.into_compile_error().into(),
    };
    match object::object(attribute, item) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

#[proc_macro_derive(TokenizeStruct)]
pub fn tokenize_struct(tokens: TokenStream) -> TokenStream {
    let input = match syn::parse(tokens) {
        Ok(item) => item,
        Err(err) => return err.into_compile_error().into(),
    };
    match method::tokenize_method_args(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

#[proc_macro_derive(DetokenizeStruct)]
pub fn detokenize_struct(tokens: TokenStream) -> TokenStream {
    let input = match syn::parse(tokens) {
        Ok(item) => item,
        Err(err) => return err.into_compile_error().into(),
    };
    match method::detokenize_method_args(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}
