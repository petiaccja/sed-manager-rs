use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{self, parse_macro_input, spanned::Spanned, DeriveInput};

struct Declaration {
    name: syn::Ident,
    variants: Vec<(syn::Ident, TokenStream2)>,
}

fn parse_variant(variant: &syn::Variant) -> Result<(syn::Ident, TokenStream2), syn::Error> {
    if variant.fields.len() != 1 {
        return Err(syn::Error::new(variant.span(), "variant must have data"));
    };
    let Some(field) = variant.fields.iter().next() else {
        return Err(syn::Error::new(variant.span(), "variant must have data"));
    };
    if field.ident.is_some() {
        return Err(syn::Error::new(field.span(), "variant data must be a single type"));
    }
    Ok((variant.ident.clone(), field.ty.to_token_stream()))
}

fn parse_declaration(input: &syn::DeriveInput) -> Result<Declaration, syn::Error> {
    let syn::Data::Enum(data) = &input.data else {
        return Err(syn::Error::new(input.span(), "expected an enum"));
    };
    let name = input.ident.clone();
    let variants: Result<Vec<_>, _> = data.variants.iter().map(|variant| parse_variant(variant)).collect();
    Ok(Declaration { name, variants: variants? })
}

fn gen_uid_patterns(declaration: &Declaration) -> TokenStream2 {
    let name = &declaration.name;
    let mut uid_patterns = quote! {};
    for variant in &declaration.variants {
        let variant_name = &variant.0;
        let variant_type = &variant.1;
        let pattern =
            quote! { #name::#variant_name(value) => <#variant_type as ::sed_manager::messaging::types::Type>::uid(), };
        uid_patterns.append_all(pattern);
    }
    uid_patterns
}

fn gen_value_patterns(declaration: &Declaration) -> TokenStream2 {
    let name = &declaration.name;
    let mut value_patterns = quote! {};
    for variant in &declaration.variants {
        let variant_name = &variant.0;
        let pattern = quote! { #name::#variant_name(value) => ::sed_manager::messaging::value::Value::from(value), };
        value_patterns.append_all(pattern);
    }
    value_patterns
}

fn gen_parse_patterns(declaration: &Declaration) -> TokenStream2 {
    let name = &declaration.name;
    let mut parse_patterns = quote! {};
    for variant in &declaration.variants {
        let variant_name = &variant.0;
        let variant_type = &variant.1;
        let pattern = quote! {
            x if x == <#variant_type as ::sed_manager::messaging::types::Type>::uid() => {
                match #variant_type::try_from(named.value) {
                    Ok(alt) => Ok(#name::#variant_name(alt)),
                    Err(named_val) => Err(::sed_manager::messaging::value::Value::from(
                        ::sed_manager::messaging::value::Named{ name: named.name, value: named_val }
                    ))
                }
            }
        };
        parse_patterns.append_all(pattern);
    }
    parse_patterns
}

fn gen_to_value(declaration: &Declaration) -> TokenStream2 {
    let name = &declaration.name;
    let uid_patterns = gen_uid_patterns(declaration);
    let value_patterns = gen_value_patterns(declaration);
    quote! {
        impl ::core::convert::From<#name> for ::sed_manager::messaging::value::Value {
            fn from(value: #name) -> Self {
                let uid = match &value {
                    #uid_patterns
                };
                let name = ::sed_manager::messaging::value::Value::from(::std::vec::Vec::from(&uid.value().to_be_bytes()[4..]));
                let value = match value {
                    #value_patterns
                };
                ::sed_manager::messaging::value::Value::from(::sed_manager::messaging::value::Named{
                    name,
                    value,
                })
            }
        }
    }
}

fn gen_try_from_alt(declaration: &Declaration) -> TokenStream2 {
    let name = &declaration.name;
    let parse_patterns = gen_parse_patterns(declaration);
    quote! {
        impl ::core::convert::TryFrom<::sed_manager::messaging::value::Value> for #name {
            type Error = ::sed_manager::messaging::value::Value;
            fn try_from(value: ::sed_manager::messaging::value::Value) -> Result<Self, Self::Error> {
                let named = ::sed_manager::messaging::value::Named::try_from(value)?;
                let Ok(bytes) = &::sed_manager::messaging::value::Bytes::try_from(&named.name) else {
                    return Err(::sed_manager::messaging::value::Value::from(named));
                };
                if bytes.len() == 4 {
                    let mut extended = [0u8; 8];
                    for i in 0..4 {
                        extended[4 + i] = bytes[i];
                    }
                    let uid = ::sed_manager::messaging::uid::UID::new(u64::from_be_bytes(extended) | 0x0000_0005_0000_0000);
                    match uid {
                        #parse_patterns
                        _ => Err(::sed_manager::messaging::value::Value::from(named)),
                    }
                }
                else {
                    Err(::sed_manager::messaging::value::Value::from(named))
                }
            }
        }
    }
}

pub fn derive_alternative_type(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as DeriveInput);
    let declaration = match parse_declaration(&input) {
        Ok(declaration) => declaration,
        Err(err) => return err.to_compile_error().into(),
    };
    let mut impls = quote! {};
    impls.append_all(gen_to_value(&declaration));
    impls.append_all(gen_try_from_alt(&declaration));
    impls.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_declaration_normal() -> Result<(), syn::Error> {
        let stream = quote! {
            enum Alt {
                OptionA(TypeA),
                OptionB(TypeB),
            }
        };
        let input = syn::parse2::<syn::DeriveInput>(stream).unwrap();
        let declaration = parse_declaration(&input)?;
        assert_eq!(declaration.name.to_string(), "Alt");
        assert_eq!(declaration.variants.len(), 2);
        assert_eq!(declaration.variants[0].0.to_string(), "OptionA");
        assert_eq!(declaration.variants[0].1.to_string(), "TypeA");
        assert_eq!(declaration.variants[1].0.to_string(), "OptionB");
        assert_eq!(declaration.variants[1].1.to_string(), "TypeB");
        Ok(())
    }
}
