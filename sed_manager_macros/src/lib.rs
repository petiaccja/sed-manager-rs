use proc_macro::TokenStream;

mod serialize;
mod types;

#[proc_macro_derive(Serialize, attributes(layout))]
pub fn derive_serialize(tokens: TokenStream) -> TokenStream {
    serialize::derive_serialize(tokens)
}

#[proc_macro_derive(Deserialize, attributes(layout))]
pub fn derive_deserialize(tokens: TokenStream) -> TokenStream {
    serialize::derive_deserialize(tokens)
}

#[proc_macro_derive(AlternativeType)]
pub fn derive_alternative_type(tokens: TokenStream) -> TokenStream {
    types::derive_alternative_type(tokens)
}
