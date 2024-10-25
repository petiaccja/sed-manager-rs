use super::parse::{FieldDesc, Layout, StructDesc};
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, TokenStreamExt};

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
        let ::core::result::Result::Ok(#struct_pos) = #stream.stream_position() else {
            return ::core::result::Result::Err(::sed_manager::serialization::SerializationError::StreamError);
        };
    }
}

fn gen_serialize_field(field: &FieldDesc) -> TokenStream2 {
    let name: TokenStream2 = field.name.parse().unwrap();
    let stream = VariableNames::stream();
    let struct_pos = VariableNames::struct_pos();
    let offset = gen_optional(field.layout.offset);
    let bits = gen_optional_range(&field.layout.bits);
    let round = gen_optional(field.layout.round);
    quote! {
        ::sed_manager::serialization::serialize::serialize_field(
            &self.#name,
            #stream,
            #struct_pos,
            #offset,
            #bits,
            #round
        )?;
    }
}

fn gen_serialize_struct_layout(layout: &Layout) -> TokenStream2 {
    if let Some(round) = layout.round {
        let round = round as u64;
        let struct_pos = VariableNames::struct_pos();
        let stream = VariableNames::stream();
        quote! {
            let ::core::result::Result::Ok(end_pos) = #stream.stream_position() else {
                return ::core::result::Result::Err(::sed_manager::serialization::SerializationError::StreamError);
            };
            let total_len = end_pos - #struct_pos;
            let rounded_len = (total_len + #round - 1) / #round * #round;
            ::sed_manager::serialization::serialize::extend_with_zeros_until(#stream, #struct_pos + rounded_len);
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
        impl ::sed_manager::serialization::Serialize<#name, u8> for #name {
            type Error = ::sed_manager::serialization::SerializationError;
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

#[cfg(test)]
mod tests {
    use quote::quote;
    use std::ops::Range;

    use super::super::parse::Layout;
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

    #[test]
    fn gen_field_serialize_simple() {
        let field = FieldDesc { name: String::from("field_n"), layout: Layout { ..Default::default() } };
        let expr = gen_serialize_field(&field);
        let expected = quote! {
            ::sed_manager::serialization::serialize::serialize_field(
                &self.field_n,
                stream,
                struct_pos,
                ::core::option::Option::None,
                ::core::option::Option::None,
                ::core::option::Option::None
            )?;
        };
        assert_eq!(expr.to_string(), expected.to_string());
    }
}
