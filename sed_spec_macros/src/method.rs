use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{
    Data, DeriveInput, Error, GenericArgument, Ident, Index, Member, PathArguments, PathSegment, Type, TypePath,
    spanned::Spanned,
};

pub fn tokenize_method_args(input: DeriveInput) -> Result<TokenStream, Error> {
    let Data::Struct(struct_item) = input.data else {
        return Err(Error::new(input.span(), "expected a struct"));
    };

    let fields = parse_fields(struct_item)?;

    let field_values = fields.iter().map(|Field { name, member, .. }| match name {
        Some(_name) => quote! {
            match &self.#member {
                Some(value) => ::sed_packet::Named{ name: #name, value }.tokenize(tokenizer),
                None => Ok(()),
            }
        },
        None => quote! { self.#member.tokenize(tokenizer) },
    });

    let ident = input.ident;

    Ok(quote! {
        #[automatically_derived]
        impl ::sed_packet::token::Tokenize for #ident {
            fn tokenize<T: ::sed_packet::token::Tokenizer>(&self, tokenizer: &mut T)
                -> ::core::result::Result<(), T::Error>
            {
                tokenizer.tokenize_list(|tokenizer| {
                    #(#field_values?;)*
                    Ok(())
                })
            }
        }
    })
}

pub fn detokenize_method_args(input: DeriveInput) -> Result<TokenStream, Error> {
    let Data::Struct(struct_item) = input.data else {
        return Err(Error::new(input.span(), "expected a struct"));
    };

    let fields = parse_fields(struct_item)?;

    let initializers = fields.iter().map(|Field { member, .. }| {
        let ident = member_to_ident(member.clone());
        quote! { let mut #ident = ::core::option::Option::None; }
    });

    let ident = &input.ident;

    let mandatory_fields = fields.iter().enumerate().filter_map(|(index, field)| {
        let ident = member_to_ident(field.member.clone());
        let ty = &field.ty;
        match &field.name {
            None => Some(quote! { #index => { #ident = Some(<#ty>::detokenize(detokenizer)?) } }),
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
                    #ident = Some(<#ty>::detokenize(detokenizer)?);
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

    Ok(quote! {
        #[automatically_derived]
        impl ::sed_packet::token::Detokenize for #ident {
            fn detokenize<D: ::sed_packet::token::Detokenizer>(detokenizer: &mut D)
                -> ::core::result::Result<Self, D::Error>
            {
                let mut index = 0usize;
                #(#initializers)*

                detokenizer.detokenize_list(|detokenizer| {
                    match index {
                        #(#mandatory_fields)*
                        _ => {
                            let _ = detokenizer.detokenize_named(
                                |detokenizer| {
                                    u16::detokenize(detokenizer)
                                },
                                |detokenizer, name| {
                                    match name {
                                        #(#optional_fields)*
                                        _ => Err(<D::Error as ::sed_packet::token::MessageError>::message("unknown optional field"))
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
            let underlying_ty = option_underlying_type(&field.ty).cloned();
            let is_optional = underlying_ty.is_some();
            if !is_optional && *name_idx != 0 {
                return Some(Err(Error::new(
                    field.ty.span(),
                    "this mandatory field must be ordered before all optional fields",
                )));
            }
            let name = is_optional.then_some(*name_idx);
            *name_idx += is_optional as u16;
            Some(Ok(Field { name, ty: underlying_ty.unwrap_or(field.ty), member }))
        })
        .collect::<Result<Vec<_>, _>>()
}

fn member_to_ident(member: Member) -> Ident {
    match member {
        Member::Named(ident) => ident,
        Member::Unnamed(Index { index, .. }) => format_ident!("m{index}"),
    }
}

fn option_underlying_type(ty: &Type) -> Option<&syn::Type> {
    match ty {
        syn::Type::Path(TypePath { qself: _, path }) => {
            let mut rev_segments = path.segments.iter().rev();
            match rev_segments.next() {
                Some(segment) if segment.ident == "Option" => (),
                _ => return None,
            };
            match rev_segments.next() {
                Some(segment) if segment.ident == "option" => (),
                None if path.leading_colon.is_none() => (),
                _ => return None,
            };
            match rev_segments.next() {
                Some(segment) if segment.ident == "std" || segment.ident == "core" => (),
                None if path.leading_colon.is_none() => (),
                _ => return None,
            };
            if let Some(PathSegment { arguments: PathArguments::AngleBracketed(args), .. }) = path.segments.last() {
                if let (Some(GenericArgument::Type(ty)), 1) = (args.args.first(), args.args.len()) {
                    return Some(ty);
                }
            };
            None
        }
        _ => None,
    }
}
