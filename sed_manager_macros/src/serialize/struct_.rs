use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, TokenStreamExt};

use crate::parse::data_struct::{DataField, DataStruct, LayoutAttr};

fn gen_optional<T: quote::ToTokens>(opt: Option<T>) -> TokenStream2 {
    match opt {
        Some(offset) => quote! { ::core::option::Option::Some(#offset)},
        None => quote! { ::core::option::Option::None },
    }
}

fn gen_optional_range<T: quote::ToTokens>(opt: &Option<core::ops::Range<T>>) -> TokenStream2 {
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
    quote! {
        use ::sed_manager::serialization::Seek;
        let struct_pos = stream.stream_position();
    }
}

fn gen_serialize_field(field: &DataField) -> TokenStream2 {
    let name = &field.name;
    let name_str = name.to_string();
    let offset = gen_optional(field.layout.offset);
    let bits = gen_optional_range(&field.layout.bits);
    let round = gen_optional(field.layout.round);
    quote! {
        ::sed_manager::serialization::annotate_field(
            ::sed_manager::serialization::serialize_field(
                &self.#name,
                stream,
                struct_pos,
                #offset,
                #bits,
                #round
            ),
            #name_str.into(),
        )?;
    }
}

fn gen_serialize_struct_layout(layout: &LayoutAttr) -> TokenStream2 {
    if let Some(round) = layout.round {
        let round = round as u64;
        quote! {
            let end_pos = stream.stream_position();
            let total_len = end_pos - struct_pos;
            let rounded_len = (total_len + #round - 1) / #round * #round;
            ::sed_manager::serialization::extend_with_zeros_until(stream, struct_pos + rounded_len);
        }
    } else {
        TokenStream2::new()
    }
}

fn gen_deserialize_struct_layout(layout: &LayoutAttr) -> TokenStream2 {
    if let Some(round) = layout.round {
        let round = round as u64;
        quote! {
            let end_pos = stream.stream_position();
            let total_len = end_pos - struct_pos;
            let rounded_len = (total_len + #round - 1) / #round * #round;
            stream.seek(::sed_manager::serialization::SeekFrom::Start(struct_pos + rounded_len))?;
        }
    } else {
        TokenStream2::new()
    }
}

fn gen_serialize_struct_skeleton(
    name: &syn::Ident,
    struct_pos: TokenStream2,
    struct_layout: TokenStream2,
    fields: TokenStream2,
) -> TokenStream2 {
    quote! {
        impl ::sed_manager::serialization::Serialize<u8> for #name {
            type Error = ::sed_manager::serialization::Error;
            fn serialize(&self, stream: &mut ::sed_manager::serialization::OutputStream<u8>) -> ::core::result::Result<(), Self::Error> {
                #struct_pos
                #fields
                #struct_layout
                ::core::result::Result::Ok(())
            }
        }
    }
}

pub fn gen_serialize_struct(struct_desc: &DataStruct) -> TokenStream2 {
    let name = &struct_desc.name;
    let struct_pos = gen_save_struct_pos();
    let mut fields = quote! {};
    for field in &struct_desc.fields {
        fields.append_all(gen_serialize_field(field));
    }
    let struct_layout = gen_serialize_struct_layout(&struct_desc.layout);
    gen_serialize_struct_skeleton(name, struct_pos, struct_layout, fields)
}

fn gen_deserialize_field(field: &DataField) -> TokenStream2 {
    let name = &field.name;
    let name_str = name.to_string();
    let ty = &field.ty;
    let offset = gen_optional(field.layout.offset);
    let bits = gen_optional_range(&field.layout.bits);
    let round = gen_optional(field.layout.round);
    quote! {
        #name: ::sed_manager::serialization::annotate_field(
            ::sed_manager::serialization::deserialize_field::<#ty>(
                stream,
                struct_pos,
                #offset,
                #bits,
                #round
            ),
            #name_str.into(),
        )?,
    }
}

fn gen_deserialize_struct_skeleton(
    name: &syn::Ident,
    struct_pos: TokenStream2,
    struct_layout: TokenStream2,
    fields: TokenStream2,
) -> TokenStream2 {
    quote! {
        impl ::sed_manager::serialization::Deserialize<u8> for #name {
            type Error = ::sed_manager::serialization::Error;
            fn deserialize(stream: &mut ::sed_manager::serialization::InputStream<u8>) -> ::core::result::Result<Self, Self::Error> {
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

pub fn gen_deserialize_struct(struct_desc: &DataStruct) -> TokenStream2 {
    let name = &struct_desc.name;
    let struct_pos = gen_save_struct_pos();
    let mut fields = quote! {};
    for field in &struct_desc.fields {
        fields.append_all(gen_deserialize_field(field));
    }
    let struct_layout = gen_deserialize_struct_layout(&struct_desc.layout);
    gen_deserialize_struct_skeleton(name, struct_pos, struct_layout, fields)
}

#[cfg(test)]
mod tests {
    use core::ops::Range;
    use quote::quote;

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
