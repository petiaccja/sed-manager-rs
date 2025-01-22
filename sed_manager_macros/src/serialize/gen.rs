use super::parse::{EnumDesc, FieldDesc, Layout, StructDesc};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, TokenStreamExt};

struct VariableNames;

impl VariableNames {
    fn struct_pos() -> TokenStream2 {
        quote! { struct_pos }
    }
    fn stream() -> TokenStream2 {
        quote! { stream }
    }
}

fn gen_optional<T: quote::ToTokens>(opt: Option<T>) -> TokenStream2 {
    match opt {
        Some(offset) => quote! { ::core::option::Option::Some(#offset)},
        None => quote! { ::core::option::Option::None },
    }
}

fn gen_optional_range<T: quote::ToTokens>(opt: &Option<std::ops::Range<T>>) -> TokenStream2 {
    match opt {
        Some(offset) => {
            let start = &offset.start;
            let end = &offset.end;
            quote! { ::core::option::Option::Some(#start .. #end) }
        }
        None => quote! { ::core::option::Option::None },
    }
}

fn gen_save_struct_pos() -> TokenStream2 {
    let struct_pos = VariableNames::struct_pos();
    let stream = VariableNames::stream();
    quote! {
        use ::std::io::Seek;
        let #struct_pos = #stream.stream_position()?;
    }
}

fn gen_serialize_field(field: &FieldDesc) -> TokenStream2 {
    let name_s = &field.name;
    let name: TokenStream2 = field.name.parse().unwrap();
    let stream = VariableNames::stream();
    let struct_pos = VariableNames::struct_pos();
    let offset = gen_optional(field.layout.offset);
    let bits = gen_optional_range(&field.layout.bits);
    let round = gen_optional(field.layout.round);
    quote! {
        ::sed_manager::serialization::annotate_field(
            ::sed_manager::serialization::serialize_field(
                &self.#name,
                #stream,
                #struct_pos,
                #offset,
                #bits,
                #round
            ),
            #name_s.into(),
        )?;
    }
}

fn gen_serialize_struct_layout(layout: &Layout) -> TokenStream2 {
    if let Some(round) = layout.round {
        let round = round as u64;
        let struct_pos = VariableNames::struct_pos();
        let stream = VariableNames::stream();
        quote! {
            let end_pos = #stream.stream_position()?;
            let total_len = end_pos - #struct_pos;
            let rounded_len = (total_len + #round - 1) / #round * #round;
            ::sed_manager::serialization::extend_with_zeros_until(#stream, #struct_pos + rounded_len);
        }
    } else {
        TokenStream2::new()
    }
}

fn gen_deserialize_struct_layout(layout: &Layout) -> TokenStream2 {
    if let Some(round) = layout.round {
        let round = round as u64;
        let struct_pos = VariableNames::struct_pos();
        let stream = VariableNames::stream();
        quote! {
            let end_pos = #stream.stream_position()?;
            let total_len = end_pos - #struct_pos;
            let rounded_len = (total_len + #round - 1) / #round * #round;
            #stream.seek(::std::io::SeekFrom::Start(#struct_pos + rounded_len))?;
        }
    } else {
        TokenStream2::new()
    }
}

fn gen_serialize_struct_skeleton(
    name: TokenStream2,
    struct_pos: TokenStream2,
    struct_layout: TokenStream2,
    fields: TokenStream2,
) -> TokenStream2 {
    let stream = VariableNames::stream();
    quote! {
        impl ::sed_manager::serialization::Serialize<u8> for #name {
            type Error = ::sed_manager::serialization::Error;
            fn serialize(&self, #stream: &mut ::sed_manager::serialization::OutputStream<u8>) -> ::core::result::Result<(), Self::Error> {
                #struct_pos
                #fields
                #struct_layout
                ::core::result::Result::Ok(())
            }
        }
    }
}

pub fn gen_serialize_struct(struct_desc: &StructDesc) -> TokenStream2 {
    let name: TokenStream2 = struct_desc.name.parse().unwrap();
    let struct_pos = gen_save_struct_pos();
    let mut fields = quote! {};
    for field in &struct_desc.fields {
        fields.append_all(gen_serialize_field(field));
    }
    let struct_layout = gen_serialize_struct_layout(&struct_desc.layout);
    gen_serialize_struct_skeleton(name, struct_pos, struct_layout, fields)
}

fn gen_deserialize_field(field: &FieldDesc) -> TokenStream2 {
    let name_s = &field.name;
    let name: TokenStream2 = field.name.parse().unwrap();
    let ty = &field.ty;
    let stream = VariableNames::stream();
    let struct_pos = VariableNames::struct_pos();
    let offset = gen_optional(field.layout.offset);
    let bits = gen_optional_range(&field.layout.bits);
    let round = gen_optional(field.layout.round);
    quote! {
        #name: ::sed_manager::serialization::annotate_field(
            ::sed_manager::serialization::deserialize_field::<#ty>(
                #stream,
                #struct_pos,
                #offset,
                #bits,
                #round
            ),
            #name_s.into(),
        )?,
    }
}

fn gen_deserialize_struct_skeleton(
    name: TokenStream2,
    struct_pos: TokenStream2,
    struct_layout: TokenStream2,
    fields: TokenStream2,
) -> TokenStream2 {
    let stream = VariableNames::stream();
    quote! {
        impl ::sed_manager::serialization::Deserialize<u8> for #name {
            type Error = ::sed_manager::serialization::Error;
            fn deserialize(#stream: &mut ::sed_manager::serialization::InputStream<u8>) -> ::core::result::Result<Self, Self::Error> {
                #struct_pos
                let value = #name {
                    #fields
                };
                #struct_layout
                ::core::result::Result::Ok(value)
            }
        }
    }
}

pub fn gen_deserialize_struct(struct_desc: &StructDesc) -> TokenStream2 {
    let name: TokenStream2 = struct_desc.name.parse().unwrap();
    let struct_pos = gen_save_struct_pos();
    let mut fields = quote! {};
    for field in &struct_desc.fields {
        fields.append_all(gen_deserialize_field(field));
    }
    let struct_layout = gen_deserialize_struct_layout(&struct_desc.layout);
    gen_deserialize_struct_skeleton(name, struct_pos, struct_layout, fields)
}

pub fn get_fallback(enum_desc: &EnumDesc) -> Option<&String> {
    enum_desc.variants.iter().find(|variant| variant.layout.fallback).map(|variant| &variant.name)
}

pub fn gen_serialize_enum(enum_desc: &EnumDesc) -> TokenStream2 {
    let name = &enum_desc.name;
    let ty = &enum_desc.ty;
    let stream = VariableNames::stream();
    let mut variants = TokenStream2::new();
    for variant in &enum_desc.variants {
        let ident = format_ident!("{}", variant.name);
        let pattern = quote! { #name::#ident => #name::#ident as #ty, };
        variants.append_all(pattern);
    }
    quote! {
        impl ::sed_manager::serialization::Serialize<u8> for #name {
            type Error = ::sed_manager::serialization::Error;
            fn serialize(&self, #stream: &mut ::sed_manager::serialization::OutputStream<u8>) -> ::core::result::Result<(), Self::Error> {
                let discr = match self {
                    #variants
                };
                match discr.serialize(#stream) {
                    ::core::result::Result::Ok(_) => ::core::result::Result::Ok(()),
                    ::core::result::Result::Err(err) => ::core::result::Result::Err(err.into()),
                }
            }
        }
    }
}

pub fn gen_deserialize_enum(enum_desc: &EnumDesc) -> TokenStream2 {
    let name = &enum_desc.name;
    let ty = &enum_desc.ty;
    let fallback = get_fallback(enum_desc).map(|name| format_ident!("{}", name));
    let stream = VariableNames::stream();
    let mut variants = TokenStream2::new();
    for variant in &enum_desc.variants {
        let ident = format_ident!("{}", variant.name);
        let pattern = quote! { x if x == (#name::#ident as #ty) => ::core::result::Result::Ok(#name::#ident), };
        variants.append_all(pattern);
    }
    let fallback_tokens = match &fallback {
        Some(variant) => quote! { ::core::result::Result::Ok(#name::#variant) },
        None => quote! { ::core::result::Result::Err(::sed_manager::serialization::Error::InvalidData) },
    };
    quote! {
        impl ::sed_manager::serialization::Deserialize<u8> for #name {
            type Error = ::sed_manager::serialization::Error;
            fn deserialize(#stream: &mut ::sed_manager::serialization::InputStream<u8>) -> ::core::result::Result<Self, Self::Error> {
                let discr = match #ty::deserialize(#stream) {
                    ::core::result::Result::Ok(discr) => discr,
                    ::core::result::Result::Err(err) => return ::core::result::Result::Err(err.into()),
                };
                match discr {
                    #variants
                    _ => #fallback_tokens,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;
    use std::ops::Range;

    use super::*;

    #[test]
    fn gen_optional_some() {
        let input = Some(234_i64);
        let expr = gen_optional(input);
        let expected = quote! { ::core::option::Option::Some(234i64) };
        assert_eq!(expr.to_string(), expected.to_string());
    }

    #[test]
    fn gen_optional_none() {
        let input: Option<i64> = None;
        let expr = gen_optional(input);
        let expected = quote! { ::core::option::Option::None };
        assert_eq!(expr.to_string(), expected.to_string());
    }

    #[test]
    fn gen_optional_range_some() {
        let input = Some(3i64..4i64);
        let expr = gen_optional_range(&input);
        let expected = quote! { ::core::option::Option::Some(3i64..4i64) };
        assert_eq!(expr.to_string(), expected.to_string());
    }

    #[test]
    fn gen_optional_range_none() {
        let input: Option<Range<i64>> = None;
        let expr = gen_optional_range(&input);
        let expected = quote! { ::core::option::Option::None };
        assert_eq!(expr.to_string(), expected.to_string());
    }
}
