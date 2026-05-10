use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Member, spanned::Spanned};

pub fn tokenize_field(input: DeriveInput) -> Result<TokenStream, Error> {
    let Data::Struct(data) = &input.data else {
        return Err(Error::new(input.span(), "expected a struct"));
    };

    let fields = data.fields.iter().enumerate().map(|(index, field)| {
        let member: Member = field.ident.clone().map(|ident| ident.into()).unwrap_or_else(|| index.into());
        let index = index as u16;
        quote! {
            #index => match &self.#member {
                ::core::option::Option::Some(value) => ::sed_packet::token::Tokenize::tokenize(
                    &::sed_packet::Named{ name: #index, value },
                    tokenizer
                ),
                ::core::option::Option::None => ::core::result::Result::Ok(()),
            }
        }
    });

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::sed_packet::TokenizeField for #name #ty_generics #where_clause {
            fn tokenize_field<T: ::sed_packet::token::Tokenizer>(
                &self,
                field: u16,
                tokenizer: &mut T,
            ) -> ::core::result::Result<(), <T as ::sed_packet::token::Tokenizer>::Error> {
                match field {
                    #(#fields,)*
                    _ => ::core::result::Result::Ok(()),
                }
            }
        }
    })
}
