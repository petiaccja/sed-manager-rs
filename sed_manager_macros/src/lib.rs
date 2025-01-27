use proc_macro::TokenStream;

mod parse;
mod serialize;
mod types;

#[proc_macro_derive(Serialize, attributes(layout, fallback))]
pub fn derive_serialize(tokens: TokenStream) -> TokenStream {
    serialize::derive_serialize(tokens)
}

#[proc_macro_derive(Deserialize, attributes(layout, fallback))]
pub fn derive_deserialize(tokens: TokenStream) -> TokenStream {
    serialize::derive_deserialize(tokens)
}

#[proc_macro_derive(AliasType)]
pub fn derive_alias_type(tokens: TokenStream) -> TokenStream {
    types::derive_alias_type(tokens)
}

#[proc_macro_derive(AlternativeType)]
pub fn derive_alternative_type(tokens: TokenStream) -> TokenStream {
    types::derive_alternative_type(tokens)
}

#[proc_macro_derive(EnumerationType, attributes(fallback))]
pub fn derive_enumeration_type(tokens: TokenStream) -> TokenStream {
    types::derive_enumeration_type(tokens)
}

#[proc_macro_derive(StructType, attributes(fallback))]
pub fn derive_struct_type(tokens: TokenStream) -> TokenStream {
    types::derive_struct_type(tokens)
}
