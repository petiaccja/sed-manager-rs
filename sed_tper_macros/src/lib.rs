use proc_macro::TokenStream;

mod method_args;

#[proc_macro_derive(TokenizeMethodArgs)]
pub fn tokenize_method_args(tokens: TokenStream) -> TokenStream {
    let input = match syn::parse(tokens) {
        Ok(item) => item,
        Err(err) => return err.into_compile_error().into(),
    };
    match method_args::tokenize_method_args(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

#[proc_macro_derive(DetokenizeMethodArgs)]
pub fn detokenize_method_args(tokens: TokenStream) -> TokenStream {
        let input = match syn::parse(tokens) {
        Ok(item) => item,
        Err(err) => return err.into_compile_error().into(),
    };
    match method_args::detokenize_method_args(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}
