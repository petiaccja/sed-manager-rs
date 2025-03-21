//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use syn::{self, spanned::Spanned};

pub struct DataEnum {
    pub name: syn::Ident,
    pub variants: Vec<DataVariant>,
}

pub struct DataVariant {
    pub name: syn::Ident,
    pub ty: syn::Type,
}

impl DataEnum {
    pub fn parse(ast: &syn::DeriveInput) -> Result<Self, syn::Error> {
        let syn::Data::Enum(enum_ast) = &ast.data else {
            return Err(syn::Error::new(ast.span(), "expected an enum"));
        };
        let name = ast.ident.clone();
        let variants: Result<Vec<_>, _> = enum_ast.variants.iter().map(|variant| DataVariant::parse(variant)).collect();
        Ok(DataEnum { name, variants: variants? })
    }
}

impl DataVariant {
    pub fn parse(ast: &syn::Variant) -> Result<Self, syn::Error> {
        let name = ast.ident.clone();
        if ast.fields.len() != 1 {
            return Err(syn::Error::new(ast.span(), "variant must have data"));
        };
        let Some(field) = ast.fields.iter().next() else {
            return Err(syn::Error::new(ast.span(), "variant must have data"));
        };
        if field.ident.is_some() {
            return Err(syn::Error::new(field.span(), "variant data must be a single type"));
        }
        let ty = field.ty.clone();
        Ok(DataVariant { name, ty })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::{quote, ToTokens};

    #[test]
    fn parse_normal() -> Result<(), syn::Error> {
        let stream = quote! {
            enum Alt {
                OptionA(TypeA),
                OptionB(TypeB),
            }
        };
        let input = syn::parse2::<syn::DeriveInput>(stream).unwrap();
        let data_enum = DataEnum::parse(&input)?;
        assert_eq!(data_enum.name.to_string(), "Alt");
        assert_eq!(data_enum.variants.len(), 2);
        assert_eq!(data_enum.variants[0].name.to_string(), "OptionA");
        assert_eq!(data_enum.variants[0].ty.to_token_stream().to_string(), "TypeA");
        assert_eq!(data_enum.variants[1].name.to_string(), "OptionB");
        assert_eq!(data_enum.variants[1].ty.to_token_stream().to_string(), "TypeB");
        Ok(())
    }
}
