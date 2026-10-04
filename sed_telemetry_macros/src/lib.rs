//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use proc_macro::TokenStream;
use quote::ToTokens;
use syn::ItemFn;

mod with_tracing;

/// Fire up a tracing subscriber before running the function.
#[proc_macro_attribute]
pub fn with_tracing(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item: ItemFn = match syn::parse(item) {
        Ok(item) => item,
        Err(err) => return err.into_compile_error().into(),
    };
    match with_tracing::with_tracing(item) {
        Ok(item) => item.to_token_stream().into(),
        Err(err) => err.into_compile_error().into(),
    }
}
