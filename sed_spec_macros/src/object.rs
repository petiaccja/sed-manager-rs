use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::{
    Attribute, Data, DataStruct, DeriveInput, Error, Expr, Member, Meta, punctuated::Punctuated, spanned::Spanned,
    token::Comma,
};

use crate::type_ext::TypeExt;

pub fn object(input: DeriveInput) -> Result<TokenStream, Error> {
    let Data::Struct(data) = &input.data else {
        return Err(Error::new(input.span(), "expected a struct"));
    };

    for field in &data.fields {
        if !field.ty.is_option() {
            return Err(Error::new(field.ty.span(), "all the fields of an object has to be an `Option`"));
        }
    }

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let table = parse_table_expr(&input.attrs, input.span())?;
    let active_fields = active_fields_method(data);
    let update = update_method(data);
    let field_indices = data.fields.iter().enumerate().filter_map(|(index, field)| match &field.ident {
        Some(ident) => {
            let const_ident = format_ident!("{}", ident.to_string().to_uppercase());
            let index = index as u16;
            Some(quote! { pub const #const_ident: u16 = #index; })
        }
        None => None,
    });
    let field_count = data.fields.len() as u16;
    Ok(quote! {
        impl #impl_generics ::sed_packet::Object for #name #ty_generics #where_clause {
            const TABLE: ::sed_packet::TableRef = #table;
            type Ref = ::sed_packet::ObjectRef<{Self::TABLE.to_u64()}>;
            const FIELD_COUNT : u16 = #field_count;
            #active_fields
            #update
        }

        impl #name {
            #(#field_indices)*
        }
    })
}

fn parse_table_expr(attrs: &[Attribute], span: Span) -> Result<Expr, Error> {
    let Some(object_attr) = attrs.iter().find(|attr| attr.path().is_ident("object")) else {
        return Err(Error::new(span, "missing attribute `object(table = <TABLEREF>)`"));
    };

    let list = object_attr.meta.require_list()?;
    let list_items = list.parse_args_with(|parse_buffer: &syn::parse::ParseBuffer<'_>| {
        Punctuated::<Meta, Comma>::parse_terminated(parse_buffer)
    })?;

    let Some(table_meta) = list_items.first() else {
        return Err(Error::new(span, "missing meta `table = <TABLEREF>`"));
    };

    let nvp = table_meta.require_name_value()?;
    if !nvp.path.is_ident("table") {
        return Err(Error::new(
            nvp.path.span(),
            format!("unexpected key `{}`, only the `table` key is accepted", nvp.path.to_token_stream()),
        ));
    }
    Ok(nvp.value.clone())
}

fn active_fields_method(item: &DataStruct) -> TokenStream {
    let fields = item.fields.iter().enumerate().map(|(index, field)| {
        let member: Member = field.ident.clone().map(|ident| ident.into()).unwrap_or_else(|| index.into());
        let index = index as u16;
        quote! {
            if self.#member.is_some() {
                result.push(#index);
            }
        }
    });
    quote! {
        fn active_fields(&self) -> ::std::vec::Vec<u16> {
            let mut result = Vec::new();
            #(#fields)*
            result
        }
    }
}

fn update_method(item: &DataStruct) -> TokenStream {
    let fields = item.fields.iter().enumerate().map(|(index, field)| {
        let member: Member = field.ident.clone().map(|ident| ident.into()).unwrap_or_else(|| index.into());
        quote! {
            if other.#member.is_some() {
                self.#member = other.#member;
            }
        }
    });
    quote! {
        fn update(&mut self, other: Self) {
            #(#fields)*
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    fn example() -> DeriveInput {
        parse_quote! {
            #[object(table = TEST)]
            struct Test {
                foo: Option<TestRef>,
                bar: Option<String>,
            }
        }
    }

    #[test]
    fn impl_object_trait_active_fields_() {
        let input = example();
        let Data::Struct(data) = input.data else { panic!() };
        let result = active_fields_method(&data);
        let expected = quote! {
            fn active_fields(&self) -> ::std::vec::Vec<u16> {
                let mut result = Vec::new();
                if self.foo.is_some() {
                    result.push(0u16);
                }
                if self.bar.is_some() {
                    result.push(1u16);
                }
                result
            }
        };
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn impl_object_trait_update_() {
        let input = example();
        let Data::Struct(data) = input.data else { panic!() };
        let result = update_method(&data);
        let expected = quote! {
            fn update(&mut self, other: Self) {
                if other.foo.is_some() {
                    self.foo = other.foo;
                }
                if other.bar.is_some() {
                    self.bar = other.bar;
                }
            }
        };
        assert_eq!(result.to_string(), expected.to_string());
    }
}
