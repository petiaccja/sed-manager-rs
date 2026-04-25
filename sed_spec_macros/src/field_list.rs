use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Error, spanned::Spanned as _};

use crate::type_ext::TypeExt as _;

pub fn field_list(input: DeriveInput) -> Result<TokenStream, Error> {
    let field_traits_for_obj = field_traits_for_obj(&input)?;
    let ref_ext = ref_ext_trait(&input)?;

    Ok(quote! {
        #field_traits_for_obj
        #ref_ext
    })
}

fn field_traits_for_obj(input: &DeriveInput) -> Result<TokenStream, Error> {
    let Data::Struct(data) = &input.data else {
        return Err(Error::new(input.span(), "expected a struct"));
    };

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let field_impls = data.fields.iter().enumerate().map(|(index, field)| {
        let index = index as u16;
        let ty = field.ty.remove_option();
        quote! {
            impl #impl_generics ::sed_packet::Field<#index> for #name #ty_generics #where_clause {
                type Type = #ty;
            }
        }
    });

    Ok(quote! { #(#field_impls)* })
}

fn ref_ext_trait(input: &DeriveInput) -> Result<TokenStream, Error> {
    let Data::Struct(data) = &input.data else {
        return Err(Error::new(input.span(), "expected a struct"));
    };

    let obj_name = &input.ident;
    let ref_name = quote! { <#obj_name as ::sed_packet::Object>::Ref };
    let ext_trait_name = format_ident!("{obj_name}RefExt");

    let getter_decls = data.fields.iter().enumerate().map(|(index, field)| {
        let index_val = index as u16;
        let index_name = format_ident!("_{index}");
        let field_name = field.ident.as_ref().unwrap_or(&index_name);
        quote! {
            fn #field_name(&self) -> ::sed_packet::FieldRef<#obj_name, { <#obj_name as ::sed_packet::Object>::TABLE.to_u64() }, #index_val>;
        }
    });

    let getter_impls = data.fields.iter().enumerate().map(|(index, field)| {
        let index_val = index as u16;
        let index_name = format_ident!("_{index}");
        let field_name = field.ident.as_ref().unwrap_or(&index_name);
        quote! {
            fn #field_name(&self) -> ::sed_packet::FieldRef<#obj_name, { <#obj_name as ::sed_packet::Object>::TABLE.to_u64() }, #index_val> {
                self.clone().into()
            }
        }
    });

    let ext_trait_decl = quote! {
        pub trait #ext_trait_name {
            #(#getter_decls)*
        }
    };

    let ext_trait_impl = quote! {
        impl #ext_trait_name for #ref_name {
            #(#getter_impls)*
        }
    };

    Ok(quote! {
        #ext_trait_decl
        #ext_trait_impl
    })
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
    fn field_traits_for_obj_() {
        let result = field_traits_for_obj(&example()).unwrap();
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
    fn ref_ext_trait_() {
        let result = ref_ext_trait(&example()).unwrap();
        let expected = quote! {
            pub trait TestRefExt {
                fn foo(&self) -> ::sed_packet::FieldRef<Test, { <Test as ::sed_packet::Object>::TABLE.to_u64() }, 0u16>;
                fn bar(&self) -> ::sed_packet::FieldRef<Test, { <Test as ::sed_packet::Object>::TABLE.to_u64() }, 1u16>;
            }

            impl TestRefExt for <Test as ::sed_packet::Object>::Ref {
                fn foo(&self) -> ::sed_packet::FieldRef<Test, { <Test as ::sed_packet::Object>::TABLE.to_u64() }, 0u16> {
                    self.clone().into()
                }

                fn bar(&self) -> ::sed_packet::FieldRef<Test, { <Test as ::sed_packet::Object>::TABLE.to_u64() }, 1u16> {
                    self.clone().into()
                }
            }
        };
        assert_eq!(result.to_string(), expected.to_string());
    }
}
