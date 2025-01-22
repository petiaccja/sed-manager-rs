use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{self, spanned::Spanned};

#[derive(Debug, Clone)]
pub struct Layout {
    pub offset: Option<usize>,
    pub bits: Option<std::ops::Range<usize>>,
    pub round: Option<usize>,
    pub fallback: bool,
}

impl Default for Layout {
    fn default() -> Self {
        Layout { offset: None, bits: None, round: None, fallback: false }
    }
}

pub struct VariantDesc {
    pub name: String,
    pub layout: Layout,
}

pub struct FieldDesc {
    pub name: String,
    pub ty: TokenStream2,
    pub layout: Layout,
}

pub struct StructDesc {
    pub name: String,
    pub layout: Layout,
    pub fields: Vec<FieldDesc>,
}

pub struct EnumDesc {
    pub name: TokenStream2,
    pub ty: TokenStream2,
    pub variants: Vec<VariantDesc>,
}

fn parse_literal_usize(expr: &syn::Expr) -> Result<usize, syn::Error> {
    let syn::Expr::Lit(lit) = expr else {
        return Err(syn::Error::new(expr.span(), "expected an integer literal"));
    };
    let syn::Lit::Int(value) = &lit.lit else {
        return Err(syn::Error::new(expr.span(), "expected an integer literal"));
    };
    value.base10_parse()
}

fn parse_literal_range_usize(expr: &syn::Expr) -> Result<std::ops::Range<usize>, syn::Error> {
    let syn::Expr::Range(range) = expr else {
        return Err(syn::Error::new(expr.span(), "expected a range expression"));
    };
    let Some(start_expr) = &range.start else {
        return Err(syn::Error::new(expr.span(), "expected a range with both start and end specified"));
    };
    let Some(end_expr) = &range.end else {
        return Err(syn::Error::new(expr.span(), "expected a range with both start and end specified"));
    };

    let start = parse_literal_usize(start_expr.as_ref())?;
    let end_value = parse_literal_usize(end_expr.as_ref())?;
    let end = match range.limits {
        syn::RangeLimits::HalfOpen(_) => end_value,
        syn::RangeLimits::Closed(_) => end_value + 1,
    };
    if start >= end {
        return Err(syn::Error::new(expr.span(), "empty range is not accepted"));
    }
    Ok(std::ops::Range::<usize> { start: start, end: end })
}

fn parse_layout_attr(attr: &syn::Attribute) -> Result<Layout, syn::Error> {
    let syn::Meta::List(list) = &attr.meta else {
        return Err(syn::Error::new(attr.span(), "expected list attribute"));
    };

    let elements = &list.tokens;
    let tuple = quote! { (#elements, ) };
    let parsed = syn::parse2::<syn::ExprTuple>(tuple)?;
    let mut layout = Layout { ..Default::default() };
    for expr in parsed.elems {
        if let syn::Expr::Assign(assign) = expr {
            let syn::Expr::Path(path) = *assign.left else {
                return Err(syn::Error::new(assign.left.span(), "expected `param = value`"));
            };
            if path.path.is_ident("offset") {
                layout.offset = Some(parse_literal_usize(&assign.right)?);
            } else if path.path.is_ident("bits") {
                layout.bits = Some(parse_literal_range_usize(&assign.right)?);
            } else if path.path.is_ident("round") {
                layout.round = Some(parse_literal_usize(&assign.right)?);
            } else {
                return Err(syn::Error::new(path.span(), "invalid layout param"));
            };
        } else if let syn::Expr::Path(path) = expr {
            if path.path.is_ident("fallback") {
                layout.fallback = true;
            } else {
                return Err(syn::Error::new(path.span(), "invalid layout param"));
            }
        } else {
            return Err(syn::Error::new(expr.span(), "invalid layout param"));
        };
    }
    Ok(layout)
}

fn find_layout_attr(attrs: &[syn::Attribute]) -> Option<&syn::Attribute> {
    for attr in attrs {
        if attr.path().is_ident("layout") {
            return Some(attr);
        }
    }
    None
}

fn parse_field(field: &syn::Field) -> Result<FieldDesc, syn::Error> {
    if let Some(ident) = &field.ident {
        let layout = match find_layout_attr(field.attrs.as_slice()) {
            Some(attr) => parse_layout_attr(attr)?.into(),
            None => Layout { ..Default::default() },
        };
        Ok(FieldDesc { name: ident.to_string(), ty: field.ty.clone().into_token_stream(), layout: layout })
    } else {
        Err(syn::Error::new(field.span(), "field must have a name"))
    }
}

pub fn parse_struct(input: &syn::DeriveInput) -> Result<StructDesc, syn::Error> {
    let syn::Data::Struct(data) = &input.data else {
        return Err(syn::Error::new(input.span(), "expected a struct"));
    };

    let name = input.ident.to_string();

    let layout = match find_layout_attr(&input.attrs) {
        Some(attr) => {
            let layout = parse_layout_attr(attr)?;
            if layout.offset.is_some() || layout.bits.is_some() {
                return Err(syn::Error::new(
                    attr.meta.span(),
                    "only the `round` layout parameter is supported for the struct",
                ));
            }
            layout
        }
        None => Layout { ..Default::default() },
    };

    let fields: Result<Vec<_>, _> = data.fields.iter().map(|field| parse_field(field)).collect();
    Ok(StructDesc { name: name, layout: layout, fields: fields? })
}

fn parse_enum_repr(attrs: &[syn::Attribute]) -> Option<TokenStream2> {
    const INTEGER_REPRS: [&str; 8] = ["u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64"];
    for attr in attrs {
        if attr.path().is_ident("repr") {
            if let syn::Meta::List(arg) = &attr.meta {
                if let Ok(ident) = syn::parse2::<syn::Ident>(arg.tokens.clone()) {
                    if INTEGER_REPRS.contains(&ident.to_string().as_str()) {
                        return Some(ident.into_token_stream());
                    }
                };
            };
        };
    }
    None
}

pub fn parse_enum_variant(variant: &syn::Variant) -> Result<VariantDesc, syn::Error> {
    let name = variant.ident.to_string();
    let layout = match find_layout_attr(variant.attrs.as_slice()) {
        Some(attr) => parse_layout_attr(attr)?.into(),
        None => Layout { ..Default::default() },
    };
    Ok(VariantDesc { name, layout })
}

pub fn parse_enum(input: &syn::DeriveInput) -> Result<EnumDesc, syn::Error> {
    let syn::Data::Enum(data) = &input.data else {
        return Err(syn::Error::new(input.span(), "expected an enum"));
    };
    let Some(ty) = parse_enum_repr(&input.attrs) else {
        return Err(syn::Error::new(input.span(), "enum must have a `#[repr(i/u*)]` attribute"));
    };
    let name = input.ident.clone().into_token_stream();
    let variants: Result<Vec<_>, _> = data.variants.iter().map(|variant| parse_enum_variant(variant)).collect();
    Ok(EnumDesc { name: name, ty: ty, variants: variants? })
}

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::DeriveInput;

    use super::*;

    #[test]
    fn parse_struct_names() {
        let stream = quote! {
            struct Data {
                pub field_a : u32,
                pub field_b : u16,
            }
        };
        let input = syn::parse2::<DeriveInput>(stream).unwrap();
        let struct_desc = parse_struct(&input).unwrap();
        assert_eq!(struct_desc.fields.len(), 2);
        assert_eq!(struct_desc.name, "Data");
        assert_eq!(struct_desc.fields[0].name, "field_a");
        assert_eq!(struct_desc.fields[1].name, "field_b");
    }

    #[test]
    fn parse_struct_top_level_attr() {
        let stream = quote! {
            #[layout(round = 20)]
            struct Data {
                pub field_a : u32,
            }
        };
        let input = syn::parse2::<DeriveInput>(stream).unwrap();
        let struct_desc = parse_struct(&input).unwrap();
        assert_eq!(struct_desc.layout.round.unwrap(), 20);
    }

    #[test]
    fn parse_struct_offset_attr() {
        let stream = quote! {
            struct Data {
                #[layout(offset=6)]
                pub field : u32,
            }
        };
        let input = syn::parse2::<DeriveInput>(stream).unwrap();
        let struct_desc = parse_struct(&input).unwrap();
        assert_eq!(struct_desc.fields.len(), 1);
        assert_eq!(struct_desc.fields[0].layout.offset.unwrap(), 6);
    }

    #[test]
    fn parse_struct_bits_attr() {
        let stream = quote! {
            struct Data {
                #[layout(bits=1..2)]
                pub field : u32,
            }
        };
        let input = syn::parse2::<DeriveInput>(stream).unwrap();
        let struct_desc = parse_struct(&input).unwrap();
        assert_eq!(struct_desc.fields.len(), 1);
        assert_eq!(struct_desc.fields[0].layout.bits.clone().unwrap(), (1..2));
    }

    #[test]
    fn parse_struct_round_attr() {
        let stream = quote! {
            struct Data {
                #[layout(round=8)]
                pub field : u32,
            }
        };
        let input = syn::parse2::<DeriveInput>(stream).unwrap();
        let struct_desc = parse_struct(&input).unwrap();
        assert_eq!(struct_desc.fields.len(), 1);
        assert_eq!(struct_desc.fields[0].layout.round.clone().unwrap(), 8);
    }

    #[test]
    fn parse_struct_multiple_attrs() {
        let stream = quote! {
            struct Data {
                #[layout(offset = 2, bits=1..2)]
                pub field : u32,
            }
        };
        let input = syn::parse2::<DeriveInput>(stream).unwrap();
        let struct_desc = parse_struct(&input).unwrap();
        assert_eq!(struct_desc.fields.len(), 1);
        assert_eq!(struct_desc.fields[0].layout.offset.unwrap(), 2);
        assert_eq!(struct_desc.fields[0].layout.bits.clone().unwrap(), (1..2));
    }

    #[test]
    fn parse_enum_no_repr() {
        let stream = quote! {
            enum Enum {
                Var1,
            }
        };
        let input = syn::parse2::<DeriveInput>(stream).unwrap();
        assert!(parse_enum(&input).is_err());
    }

    #[test]
    fn parse_enum_simple() {
        let stream = quote! {
            #[repr(u16)]
            enum Enum {
                Var1,
                #[layout(fallback)]
                Var2,
            }
        };
        let input = syn::parse2::<DeriveInput>(stream).unwrap();
        let desc = parse_enum(&input).unwrap();
        assert_eq!(desc.name.to_string(), quote! {Enum}.to_string());
        assert_eq!(desc.ty.to_string(), quote! {u16}.to_string());
        assert_eq!(desc.variants.len(), 2);
        assert_eq!(desc.variants[0].name, "Var1");
        assert_eq!(desc.variants[1].name, "Var2");
        assert_eq!(desc.variants[1].layout.fallback, true);
    }
}
