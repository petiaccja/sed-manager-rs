//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};

use crate::parse::{
    data_struct::{DataField, DataStruct, LayoutAttr},
    ByteOrder,
};

fn gen_optional<T: quote::ToTokens>(opt: Option<T>) -> TokenStream2 {
    match opt {
        Some(offset) => quote! { ::core::option::Option::Some(#offset)},
        None => quote! { ::core::option::Option::None },
    }
}

fn gen_range<T: quote::ToTokens>(range: &core::ops::Range<T>) -> TokenStream2 {
    let start = &range.start;
    let end = &range.end;
    quote! { (#start .. #end) }
}

fn gen_save_struct_pos() -> TokenStream2 {
    quote! {
        use ::sed_manager::serialization::Seek;
        let struct_pos = stream.stream_position();
    }
}

fn gen_byte_order_expr(byte_order: ByteOrder) -> TokenStream2 {
    match byte_order {
        ByteOrder::Inherit => quote! { stream.byte_order },
        ByteOrder::BigEndian => quote! { ::sed_manager::serialization::ByteOrder::BigEndian },
        ByteOrder::LittleEndian => quote! { ::sed_manager::serialization::ByteOrder::LittleEndian },
    }
}

fn gen_serialize_field(field: &DataField) -> TokenStream2 {
    let name = &field.name;
    let name_str = name.to_string();
    let offset = gen_optional(field.layout.offset);
    let round = gen_optional(field.layout.round);
    let byte_order = gen_byte_order_expr(field.layout.byte_order);
    let serialize_expr = if let Some(bit_field) = &field.layout.bit_field {
        let bit_field_ty = &bit_field.ty;
        let bits = gen_range(&bit_field.bits);
        quote! { ::sed_manager::serialization::field::serialize_bit_field::<_, #bit_field_ty>(
            stream, &self.#name, some_struct_pos, #offset, #round, #bits
        ) }
    } else {
        quote! { ::sed_manager::serialization::field::serialize(
            stream, &self.#name, some_struct_pos, #offset, #round
        ) }
    };
    quote! {
        {
            let some_struct_pos = ::core::option::Option::Some(struct_pos);
            let byte_order = #byte_order;
            let result = stream.with_byte_order(byte_order, |stream| #serialize_expr);
            ::sed_manager::serialization::annotate_field(result, #name_str.into())?;
        }
    }
}

fn gen_serialize_struct_layout(layout: &LayoutAttr) -> TokenStream2 {
    if let Some(round) = layout.round {
        let round = round as u64;
        quote! {
            let end_pos = stream.stream_position();
            let total_len = end_pos - struct_pos;
            let rounded_len = (total_len + #round - 1) / #round * #round;
            ::sed_manager::serialization::field::extend_with_zeros_until(stream, struct_pos + rounded_len);
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
    byte_order: ByteOrder,
    struct_pos: TokenStream2,
    struct_layout: TokenStream2,
    fields: TokenStream2,
) -> TokenStream2 {
    let byte_order = gen_byte_order_expr(byte_order);
    quote! {
        impl ::sed_manager::serialization::Serialize<u8> for #name {
            type Error = ::sed_manager::serialization::Error;
            fn serialize(&self, stream: &mut ::sed_manager::serialization::OutputStream<u8>) -> ::core::result::Result<(), Self::Error> {
                let byte_order = #byte_order;
                stream.with_byte_order(byte_order, |stream| {
                    #struct_pos
                    #fields
                    #struct_layout
                    ::core::result::Result::Ok(())
                })
            }
        }
    }
}

pub fn gen_serialize_struct(struct_desc: &DataStruct) -> TokenStream2 {
    let name = &struct_desc.name;
    let byte_order = struct_desc.layout.byte_order;
    let struct_pos = gen_save_struct_pos();
    let mut fields = quote! {};
    for field in &struct_desc.fields {
        fields.append_all(gen_serialize_field(field));
    }
    let struct_layout = gen_serialize_struct_layout(&struct_desc.layout);
    gen_serialize_struct_skeleton(name, byte_order, struct_pos, struct_layout, fields)
}

fn gen_deserialize_field(field: &DataField) -> TokenStream2 {
    let name = &field.name;
    let name_str = name.to_string();
    let ty = &field.ty;
    let offset = gen_optional(field.layout.offset);
    let round = gen_optional(field.layout.round);
    let byte_order = gen_byte_order_expr(field.layout.byte_order);

    let deserialize_expr = if let Some(bit_field) = &field.layout.bit_field {
        let bit_field_ty = &bit_field.ty;
        let bits = gen_range(&bit_field.bits);
        if ty.to_token_stream().to_string() != "bool" {
            quote! { ::sed_manager::serialization::field::deserialize_bit_field::<#ty, #bit_field_ty>(
                stream, some_struct_pos, #offset, #round, #bits
            ) }
        } else {
            quote! { ::sed_manager::serialization::field::deserialize_bit_field_bool::<#bit_field_ty>(
                stream, some_struct_pos, #offset, #round, #bits
            ) }
        }
    } else {
        quote! { ::sed_manager::serialization::field::deserialize::<#ty>(
            stream, some_struct_pos, #offset, #round
        ) }
    };

    quote! {
        #name: {
            let some_struct_pos = ::core::option::Option::Some(struct_pos);
            let byte_order = #byte_order;
            let result = stream.with_byte_order(byte_order, |stream| { #deserialize_expr });
            ::sed_manager::serialization::annotate_field(result, #name_str.into())?
        },
    }
}

fn gen_deserialize_struct_skeleton(
    name: &syn::Ident,
    byte_order: ByteOrder,
    struct_pos: TokenStream2,
    struct_layout: TokenStream2,
    fields: TokenStream2,
) -> TokenStream2 {
    let byte_order = gen_byte_order_expr(byte_order);
    quote! {
        impl ::sed_manager::serialization::Deserialize<u8> for #name {
            type Error = ::sed_manager::serialization::Error;
            fn deserialize(stream: &mut ::sed_manager::serialization::InputStream<u8>) -> ::core::result::Result<Self, Self::Error> {
                let byte_order = #byte_order;
                stream.with_byte_order(byte_order, |stream| {
                    #struct_pos
                    let value = #name {
                        #fields
                    };
                    #struct_layout
                    ::core::result::Result::Ok(value)
                })
            }
        }
    }
}

pub fn gen_deserialize_struct(struct_desc: &DataStruct) -> TokenStream2 {
    let name = &struct_desc.name;
    let byte_order = struct_desc.layout.byte_order;
    let struct_pos = gen_save_struct_pos();
    let mut fields = quote! {};
    for field in &struct_desc.fields {
        fields.append_all(gen_deserialize_field(field));
    }
    let struct_layout = gen_deserialize_struct_layout(&struct_desc.layout);
    gen_deserialize_struct_skeleton(name, byte_order, struct_pos, struct_layout, fields)
}

#[cfg(test)]
mod tests {
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
}
