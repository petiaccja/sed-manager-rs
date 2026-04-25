mod field_list;
mod object;
mod r#struct;
mod type_ext;

use proc_macro::TokenStream;

#[proc_macro_derive(Object, attributes(object))]
pub fn object(tokens: TokenStream) -> TokenStream {
    let item = match syn::parse(tokens) {
        Ok(item) => item,
        Err(err) => return err.into_compile_error().into(),
    };
    match object::object(item) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

#[proc_macro_derive(FieldList)]
pub fn field_list(tokens: TokenStream) -> TokenStream {
    let item = match syn::parse(tokens) {
        Ok(item) => item,
        Err(err) => return err.into_compile_error().into(),
    };
    match field_list::field_list(item) {
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
    match r#struct::tokenize_struct(input) {
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
    match r#struct::detokenize_struct(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}
