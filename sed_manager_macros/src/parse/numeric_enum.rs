use syn::{self, spanned::Spanned};

pub struct NumericEnum {
    pub name: syn::Ident,
    pub repr: syn::Ident,
    pub variants: Vec<NumericVariant>,
    pub fallback: Option<syn::Ident>,
}

pub struct NumericVariant {
    pub name: syn::Ident,
}

impl NumericEnum {
    pub fn parse(ast: &syn::DeriveInput) -> Result<Self, syn::Error> {
        let syn::Data::Enum(enum_ast) = &ast.data else {
            return Err(syn::Error::new(ast.span(), "expected an enum"));
        };
        let Some(repr) = get_repr(&ast.attrs) else {
            return Err(syn::Error::new(ast.span(), "enum must have a `#[repr(i/u*)]` attribute"));
        };
        let name = ast.ident.clone();
        let variants: Result<Vec<_>, _> =
            enum_ast.variants.iter().map(|variant| NumericVariant::parse(variant)).collect();
        check_invalid_attrs(enum_ast.variants.iter())?;
        let fallback = get_fallback(enum_ast.variants.iter());
        Ok(NumericEnum { name, repr, variants: variants?, fallback })
    }
}

impl NumericVariant {
    pub fn parse(ast: &syn::Variant) -> Result<Self, syn::Error> {
        let name = ast.ident.clone();
        Ok(NumericVariant { name })
    }
}

fn get_repr(attrs: &[syn::Attribute]) -> Option<syn::Ident> {
    const INTEGER_REPRS: [&str; 8] = ["u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64"];
    for attr in attrs {
        if attr.path().is_ident("repr") {
            if let syn::Meta::List(arg) = &attr.meta {
                if let Ok(ident) = syn::parse2::<syn::Ident>(arg.tokens.clone()) {
                    if INTEGER_REPRS.contains(&ident.to_string().as_str()) {
                        return Some(ident);
                    }
                };
            };
        };
    }
    None
}

fn get_fallback<'a>(variants: impl Iterator<Item = &'a syn::Variant>) -> Option<syn::Ident> {
    for variant in variants {
        let fallback = variant.attrs.iter().any(|attr| attr.path().is_ident("fallback"));
        if fallback {
            return Some(variant.ident.clone());
        }
    }
    None
}

fn check_invalid_attrs<'a>(variants: impl Iterator<Item = &'a syn::Variant>) -> Result<(), syn::Error> {
    for variant in variants {
        if variant.attrs.iter().any(|attr| attr.path().is_ident("layout")) {
            return Err(syn::Error::new(variant.span(), "invalid attribute: `layout`"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn parse_enum_no_repr() {
        let stream = quote! {
            enum Enum {
                Var1,
            }
        };
        let input = syn::parse2::<syn::DeriveInput>(stream).unwrap();
        assert!(NumericEnum::parse(&input).is_err());
    }

    #[test]
    fn parse_enum_simple() {
        let stream = quote! {
            #[repr(u16)]
            enum Enum {
                Var1,
                #[fallback]
                Var2,
            }
        };
        let input = syn::parse2::<syn::DeriveInput>(stream).unwrap();
        let desc = NumericEnum::parse(&input).unwrap();
        assert_eq!(desc.name.to_string(), quote! {Enum}.to_string());
        assert_eq!(desc.repr.to_string(), quote! {u16}.to_string());
        assert_eq!(desc.variants.len(), 2);
        assert_eq!(desc.variants[0].name, "Var1");
        assert_eq!(desc.variants[1].name, "Var2");
        assert_eq!(desc.fallback.unwrap().to_string(), "Var2");
    }
}
