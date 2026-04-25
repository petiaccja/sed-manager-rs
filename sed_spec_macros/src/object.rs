use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Error, ItemStruct, Member, Meta, parse_quote, spanned::Spanned};

pub fn object(attribute: Meta, item: ItemStruct) -> Result<TokenStream, Error> {
    let field_ref_getters = impl_field_ref_getters(&item)?;
    let fields = impl_fields(&item);
    let tokenize = impl_tokenize(&item);
    let detokenize = impl_detokenize(&item);
    let object_trait = impl_object_trait(&attribute, &item)?;
    let object_definition = make_fields_optional(item);
    Ok(quote! {
        #object_definition
        #field_ref_getters
        #fields
        #tokenize
        #detokenize
        #object_trait
    })
}

fn make_fields_optional(mut item: ItemStruct) -> ItemStruct {
    for field in &mut item.fields {
        let ty = &field.ty;
        let new_ty = parse_quote! { ::core::option::Option<#ty> };
        field.ty = new_ty;
    }
    item
}

fn impl_field_ref_getters(item: &ItemStruct) -> Result<TokenStream, Error> {
    let self_ty = &item.ident;
    let ref_ty = &item
        .fields
        .iter()
        .next()
        .ok_or_else(|| Error::new(item.span(), "the first field of the struct must be present and be the UID"))?
        .ty;
    let getters = item.fields.iter().enumerate().map(|(index, field)| {
        let index = index as u16;
        let numeric_ident = format_ident!("field_{index}");
        let ident = field.ident.as_ref().unwrap_or(&numeric_ident);
        quote! {
            pub const fn #ident(object: #ref_ty) -> ::sed_packet::FieldRef<Self, {<Self as ::sed_packet::Object>::TABLE.to_u64()}, #index> {
                ::sed_packet::FieldRef::new(object)
            }
        }
    });
    Ok(quote! {
        impl #self_ty {
            #(#getters)*
        }
    })
}

fn impl_fields(item: &ItemStruct) -> TokenStream {
    let self_ty = &item.ident;
    let fields = item.fields.iter().enumerate().map(|(index, field)| {
        let index = index as u16;
        let ty = &field.ty;
        quote! {
            impl ::sed_packet::Field<#index> for #self_ty {
                type Type = #ty;
            }
        }
    });
    quote! {
        #(#fields)*
    }
}

fn impl_tokenize(item: &ItemStruct) -> TokenStream {
    let self_ty = &item.ident;
    let fields = item.fields.iter().enumerate().map(|(index, field)| {
        let member: Member = field.ident.clone().map(|ident| ident.into()).unwrap_or_else(|| index.into());
        let index = index as u16;
        quote! {
            self.#member
                .as_ref()
                .map(|value| ::sed_packet::Named { name: #index, value }.tokenize(tokenizer))
                .transpose()?
        }
    });
    quote! {
        impl ::sed_packet::token::Tokenize for #self_ty {
            fn tokenize<T: ::sed_packet::token::Tokenizer>(&self, tokenizer: &mut T) -> ::core::result::Result<(), T::Error> {
                tokenizer.tokenize_list(|tokenizer| {
                    #(#fields;)*
                    Ok(())
                })
            }
        }
    }
}

fn impl_detokenize(item: &ItemStruct) -> TokenStream {
    let self_ty = &item.ident;
    let fields = item.fields.iter().enumerate().map(|(index, field)| {
        let member: Member = field.ident.clone().map(|ident| ident.into()).unwrap_or_else(|| index.into());
        let ty = &field.ty;
        let index = index as u16;
        quote! {
            #index => {
                result.#member = Some(<#ty>::detokenize(de)?);
                Ok(())
            }
        }
    });
    quote! {
        impl ::sed_packet::token::Detokenize for #self_ty {
            fn detokenize<D: ::sed_packet::token::Detokenizer>(detokenizer: &mut D) -> ::core::result::Result<Self, D::Error> {
                let mut result = Self::default();
                detokenizer.detokenize_list(|de| {
                    de.detokenize_named(
                        |de| u16::detokenize(de),
                        |de, field| match field {
                            #(#fields)*
                            _ => Err(<D::Error as ::sed_packet::token::MessageError>::message("invalid field")),
                        },
                    )?;
                    Ok(())
                })?;
                Ok(result)
            }
        }
    }
}

fn impl_object_trait_table(attribute: &Meta) -> Result<TokenStream, Error> {
    let nv = attribute.require_name_value()?;
    if !nv.path.is_ident("table") {
        return Err(Error::new(attribute.span(), "please specify `table=<TableRef>`"));
    }
    let table = &nv.value;
    Ok(quote! { const TABLE: ::sed_packet::TableRef = #table; })
}

fn impl_object_trait_active_fields(item: &ItemStruct) -> TokenStream {
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

fn impl_object_trait_update(item: &ItemStruct) -> TokenStream {
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

fn impl_object_trait(attribute: &Meta, item: &ItemStruct) -> Result<TokenStream, Error> {
    let self_ty = &item.ident;
    let table = impl_object_trait_table(attribute)?;
    let active_fields = impl_object_trait_active_fields(item);
    let update = impl_object_trait_update(item);
    Ok(quote! {
        impl ::sed_packet::Object for #self_ty {
            #table
            type Ref = ::sed_packet::ObjectRef<{Self::TABLE.to_u64()}>;
            #active_fields
            #update
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example() -> ItemStruct {
        parse_quote! {
            struct Test {
                foo: TestRef,
                bar: String,
            }
        }
    }

    #[test]
    fn make_fields_optional_() {
        let result = make_fields_optional(example());
        let expected = parse_quote! {
            struct Test {
                foo: ::core::option::Option<TestRef>,
                bar: ::core::option::Option<String>,
            }
        };
        assert_eq!(result, expected);
    }

    #[test]
    fn impl_field_ref_getters_() {
        let result = impl_field_ref_getters(&example()).unwrap();
        let expected = quote! {
            impl Test {
                const fn foo(object: TestRef) -> ::sed_packet::FieldRef<Self, { <Self as ::sed_packet::Object>::TABLE.to_u64 () }, 0u16> {
                    ::sed_packet::FieldRef::new(object)
                }

                const fn bar(object: TestRef) -> ::sed_packet::FieldRef<Self, { <Self as ::sed_packet::Object>::TABLE.to_u64 () }, 1u16> {
                    ::sed_packet::FieldRef::new(object)
                }
            }
        };
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn impl_fields_() {
        let result = impl_fields(&example());
        let expected = quote! {
            impl ::sed_packet::Field<0u16> for Test {
                type Type = TestRef;
            }

            impl ::sed_packet::Field<1u16> for Test {
                type Type = String;
            }
        };
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn impl_tokenize_() {
        let result = impl_tokenize(&example());
        let expected = quote! {
            impl ::sed_packet::token::Tokenize for Test {
                fn tokenize<T: ::sed_packet::token::Tokenizer>(&self, tokenizer: &mut T) -> ::core::result::Result<(), T::Error> {
                    tokenizer.tokenize_list(|tokenizer| {
                        self.foo
                            .as_ref()
                            .map(|value| ::sed_packet::Named { name: 0u16, value }.tokenize(tokenizer))
                            .transpose()?;
                        self.bar
                            .as_ref()
                            .map(|value| ::sed_packet::Named { name: 1u16, value }.tokenize(tokenizer))
                            .transpose()?;
                        Ok(())
                    })
                }
            }
        };
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn impl_detokenize_() {
        let result = impl_detokenize(&example());
        let expected = quote! {
            impl ::sed_packet::token::Detokenize for Test {
                fn detokenize<D: ::sed_packet::token::Detokenizer>(detokenizer: &mut D) -> ::core::result::Result<Self, D::Error> {
                    let mut result = Self::default();
                    detokenizer.detokenize_list(|de| {
                        de.detokenize_named(
                            |de| u16::detokenize(de),
                            |de, field| match field {
                                0u16 => {
                                    result.foo = Some(TestRef::detokenize(de)?);
                                    Ok(())
                                }
                                1u16 => {
                                    result.bar = Some(String::detokenize(de)?);
                                    Ok(())
                                }
                                _ => Err(D::Error::message("invalid field")),
                            },
                        )?;
                        Ok(())
                    })?;
                    Ok(result)
                }
            }
        };
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn impl_object_trait_active_fields_() {
        let result = impl_object_trait_active_fields(&example());
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
        let result = impl_object_trait_update(&example());
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
