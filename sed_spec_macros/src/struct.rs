use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{Data, DeriveInput, Error, Ident, Index, Member, Type, spanned::Spanned as _};

use crate::type_ext::TypeExt;

pub fn tokenize_struct(input: DeriveInput) -> Result<TokenStream, Error> {
    let Data::Struct(struct_item) = input.data else {
        return Err(Error::new(input.span(), "expected a struct"));
    };

    let fields = parse_fields(struct_item)?;

    let field_values = fields.iter().map(|Field { name, member, .. }| match name {
        Some(_name) => quote! {
            match &self.#member {
                Some(value) => ::sed_packet::Named{ name: #name, value }.tokenize(__tokenizer),
                None => Ok(()),
            }
        },
        None => quote! { self.#member.tokenize(__tokenizer) },
    });

    let ident = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics ::sed_packet::token::Tokenize for #ident #ty_generics #where_clause {
            fn tokenize<T: ::sed_packet::token::Tokenizer>(&self, __tokenizer: &mut T)
                -> ::core::result::Result<(), T::Error>
            {
                __tokenizer.tokenize_list(|__tokenizer| {
                    #(#field_values?;)*
                    Ok(())
                })
            }
        }
    })
}

pub fn detokenize_struct(input: DeriveInput) -> Result<TokenStream, Error> {
    let Data::Struct(struct_item) = input.data else {
        return Err(Error::new(input.span(), "expected a struct"));
    };

    let fields = parse_fields(struct_item)?;

    let initializers = fields.iter().map(|Field { member, .. }| {
        let ident = member_to_ident(member.clone());
        quote! { let mut #ident = ::core::option::Option::None; }
    });

    let mandatory_fields = fields.iter().enumerate().filter_map(|(index, field)| {
        let ident = member_to_ident(field.member.clone());
        let ty = &field.ty;
        match &field.name {
            None => Some(quote! { #index => { #ident = Some(<#ty>::detokenize(__detokenizer)?) } }),
            Some(_) => None,
        }
    });

    let optional_fields = fields.iter().filter_map(|field| {
        let ident = member_to_ident(field.member.clone());
        let ty = &field.ty;
        match field.name {
            None => None,
            Some(name) => Some(quote! {
                #name => {
                    #ident = Some(<#ty>::detokenize(__detokenizer)?);
                    Ok(())
                }
            }),
        }
    });

    let construct = fields.iter().map(|Field { name, member, .. }| {
        let ident = member_to_ident(member.clone());
        let message = format!("mandatory field {} missing", member.to_token_stream().to_string());
        match name {
            Some(_) => quote! { #member: #ident },
            None => quote! {
                #member: #ident.ok_or_else(|| <D::Error as ::sed_packet::token::MessageError>::message(#message))?
            },
        }
    });

    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics ::sed_packet::token::Detokenize  for #ident  #ty_generics #where_clause {
            fn detokenize<D: ::sed_packet::token::Detokenizer>(__detokenizer: &mut D)
                -> ::core::result::Result<Self, D::Error>
            {
                let mut index = 0usize;
                #(#initializers)*

                __detokenizer.detokenize_list(|__detokenizer| {
                    match index {
                        #(#mandatory_fields)*
                        _ => {
                            let _ = __detokenizer.detokenize_named(
                                |__detokenizer| {
                                    u16::detokenize(__detokenizer)
                                },
                                |__detokenizer, __name| {
                                    match __name {
                                        #(#optional_fields)*
                                        _ => ::core::result::Result::<(), D::Error>::Err(<D::Error as ::sed_packet::token::MessageError>::message("unknown optional field"))
                                    }
                                }
                            )?;
                        }
                    };
                    index += 1;
                    Ok(())
                })?;

                Ok(#ident {
                    #(#construct,)*
                })
            }
        }
    })
}

struct Field {
    name: Option<u16>,
    ty: Type,
    member: Member,
}

fn parse_fields(struct_item: syn::DataStruct) -> Result<Vec<Field>, Error> {
    struct_item
        .fields
        .into_iter()
        .enumerate()
        .scan(0u16, |name_idx, (index, field)| {
            let member = match field.ident {
                Some(ident) => Member::from(ident),
                None => Member::from(index as usize),
            };
            let is_optional = field.ty.is_option();
            if !is_optional && *name_idx != 0 {
                return Some(Err(Error::new(
                    field.ty.span(),
                    "this mandatory field must be ordered before all optional fields",
                )));
            }
            let name = is_optional.then_some(*name_idx);
            *name_idx += is_optional as u16;
            Some(Ok(Field { name, ty: field.ty.remove_option().clone(), member }))
        })
        .collect::<Result<Vec<_>, _>>()
}

fn member_to_ident(member: Member) -> Ident {
    match member {
        Member::Named(ident) => ident,
        Member::Unnamed(Index { index, .. }) => format_ident!("m{index}"),
    }
}
