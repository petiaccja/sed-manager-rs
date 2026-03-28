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
