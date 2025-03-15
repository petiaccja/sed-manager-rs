use quote::{quote, ToTokens};
use syn::{self, spanned::Spanned};

use super::ByteOrder;

pub struct DataStruct {
    pub name: syn::Ident,
    pub fields: Vec<DataField>,
    pub layout: LayoutAttr,
}

pub struct DataField {
    pub name: syn::Ident,
    pub ty: syn::Type,
    pub layout: LayoutAttr,
}

pub struct BitField {
    pub bits: core::ops::Range<u8>,
    pub ty: syn::Type,
}

pub struct LayoutAttr {
    pub byte_order: ByteOrder,
    pub offset: Option<u64>,
    pub round: Option<u64>,
    pub bit_field: Option<BitField>,
}

impl DataStruct {
    pub fn parse(ast: &syn::DeriveInput) -> Result<Self, syn::Error> {
        let syn::Data::Struct(data) = &ast.data else {
            return Err(syn::Error::new(ast.span(), "expected a struct"));
        };

        let name = ast.ident.clone();

        let layout = match find_layout_attr(&ast.attrs) {
            Some(attr) => {
                let layout = LayoutAttr::parse(attr)?;
                if layout.offset.is_some() {
                    return Err(syn::Error::new(
                        attr.meta.span(),
                        "the `offset` layout parameter is not allowed on a struct",
                    ));
                }
                if layout.bit_field.is_some() {
                    return Err(syn::Error::new(
                        attr.meta.span(),
                        "the `bit_field` layout parameter is not allowed on a struct",
                    ));
                }
                layout
            }
            None => LayoutAttr { ..Default::default() },
        };

        let fields: Result<Vec<_>, _> = data.fields.iter().map(|field| DataField::parse(field)).collect();
        Ok(Self { name, fields: fields?, layout })
    }
}

impl DataField {
    pub fn parse(ast: &syn::Field) -> Result<Self, syn::Error> {
        if let Some(ident) = &ast.ident {
            let layout = match find_layout_attr(ast.attrs.as_slice()) {
                Some(attr) => LayoutAttr::parse(attr)?,
                None => LayoutAttr { ..Default::default() },
            };
            Ok(Self { name: ident.clone(), ty: ast.ty.clone(), layout: layout })
        } else {
            Err(syn::Error::new(ast.span(), "field must have a name"))
        }
    }
}

impl LayoutAttr {
    pub fn parse(ast: &syn::Attribute) -> Result<Self, syn::Error> {
        let syn::Meta::List(list) = &ast.meta else {
            return Err(syn::Error::new(ast.span(), "expected list attribute"));
        };

        let elements = &list.tokens;
        let tuple = quote! { (#elements, ) };
        let parsed = syn::parse2::<syn::ExprTuple>(tuple)?;
        let mut layout = LayoutAttr { ..Default::default() };
        for expr in parsed.elems {
            if let syn::Expr::Assign(assign) = expr {
                let syn::Expr::Path(path) = *assign.left else {
                    return Err(syn::Error::new(assign.left.span(), "expected `param = value`"));
                };
                if path.path.is_ident("offset") {
                    layout.offset = Some(parse_literal_u64(&assign.right)?);
                } else if path.path.is_ident("round") {
                    layout.round = Some(parse_literal_u64(&assign.right)?);
                } else {
                    return Err(syn::Error::new(
                        path.span(),
                        format!("invalid layout param `{}`", path.path.to_token_stream()),
                    ));
                };
            } else if let syn::Expr::Call(call) = expr {
                let syn::Expr::Path(name) = *call.func else {
                    return Err(syn::Error::new(call.func.span(), "expected `param(values...)`"));
                };
                if name.path.is_ident("bit_field") {
                    let (Some(ty), Some(bits)) = (call.args.get(0), call.args.get(1)) else {
                        return Err(syn::Error::new(call.args.span(), "expected `bit_field(type, bits)`"));
                    };
                    let ty = syn::parse2::<syn::Type>(ty.into_token_stream())?;
                    let bits = if let Ok(range) = parse_literal_range_u8(bits) {
                        range
                    } else {
                        let single_bit = parse_literal_u8(bits)?;
                        single_bit..(single_bit + 1)
                    };
                    layout.bit_field = Some(BitField { bits, ty });
                } else {
                    return Err(syn::Error::new(
                        name.span(),
                        format!("invalid layout param `{}`", name.to_token_stream().to_string()),
                    ));
                }
            } else if let syn::Expr::Path(path) = expr {
                if path.path.is_ident("big_endian") {
                    layout.byte_order = ByteOrder::BigEndian;
                } else if path.path.is_ident("little_endian") {
                    layout.byte_order = ByteOrder::LittleEndian;
                } else {
                    return Err(syn::Error::new(
                        path.span(),
                        format!("invalid layout param `{}`", path.to_token_stream().to_string()),
                    ));
                }
            } else {
                return Err(syn::Error::new(expr.span(), "invalid layout param"));
            };
        }
        Ok(layout)
    }
}

impl Default for LayoutAttr {
    fn default() -> Self {
        LayoutAttr { byte_order: ByteOrder::Inherit, offset: None, bit_field: None, round: None }
    }
}

fn parse_literal_u64(expr: &syn::Expr) -> Result<u64, syn::Error> {
    let syn::Expr::Lit(literal) = expr else {
        return Err(syn::Error::new(expr.span(), "expected an integer literal"));
    };
    let syn::Lit::Int(integral) = &literal.lit else {
        return Err(syn::Error::new(expr.span(), "expected an integer literal"));
    };
    integral.base10_parse()
}

fn parse_literal_u8(expr: &syn::Expr) -> Result<u8, syn::Error> {
    let syn::Expr::Lit(literal) = expr else {
        return Err(syn::Error::new(expr.span(), "expected an integer literal"));
    };
    let syn::Lit::Int(integral) = &literal.lit else {
        return Err(syn::Error::new(expr.span(), "expected an integer literal"));
    };
    integral.base10_parse()
}

fn parse_literal_range_u8(expr: &syn::Expr) -> Result<core::ops::Range<u8>, syn::Error> {
    let syn::Expr::Range(range) = expr else {
        return Err(syn::Error::new(expr.span(), "expected a range expression"));
    };
    let Some(start_expr) = &range.start else {
        return Err(syn::Error::new(expr.span(), "expected a range with both start and end specified"));
    };
    let Some(end_expr) = &range.end else {
        return Err(syn::Error::new(expr.span(), "expected a range with both start and end specified"));
    };

    let start = parse_literal_u8(start_expr.as_ref())?;
    let end_value = parse_literal_u8(end_expr.as_ref())?;
    let end = match range.limits {
        syn::RangeLimits::HalfOpen(_) => end_value,
        syn::RangeLimits::Closed(_) => end_value + 1,
    };
    if start >= end {
        return Err(syn::Error::new(expr.span(), "empty range is not accepted"));
    }
    Ok(core::ops::Range::<u8> { start, end })
}

fn find_layout_attr(attrs: &[syn::Attribute]) -> Option<&syn::Attribute> {
    for attr in attrs {
        if attr.path().is_ident("layout") {
            return Some(attr);
        }
    }
    None
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
        let struct_desc = DataStruct::parse(&input).unwrap();
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
        let struct_desc = DataStruct::parse(&input).unwrap();
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
        let struct_desc = DataStruct::parse(&input).unwrap();
        assert_eq!(struct_desc.fields.len(), 1);
        assert_eq!(struct_desc.fields[0].layout.offset.unwrap(), 6);
    }

    #[test]
    fn parse_struct_bit_field_attr() {
        let stream = quote! {
            struct Data {
                #[layout(bit_field(u16, 1..2))]
                pub field : u32,
            }
        };
        let input = syn::parse2::<DeriveInput>(stream).unwrap();
        let struct_desc = DataStruct::parse(&input).unwrap();
        assert_eq!(struct_desc.fields.len(), 1);
        assert_eq!(struct_desc.fields[0].layout.bit_field.as_ref().unwrap().bits, (1..2));
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
        let struct_desc = DataStruct::parse(&input).unwrap();
        assert_eq!(struct_desc.fields.len(), 1);
        assert_eq!(struct_desc.fields[0].layout.round.clone().unwrap(), 8);
    }

    #[test]
    fn parse_struct_multiple_attrs() {
        let stream = quote! {
            struct Data {
                #[layout(offset = 2, bit_field(u16, 1..2))]
                pub field : u32,
            }
        };
        let input = syn::parse2::<DeriveInput>(stream).unwrap();
        let struct_desc = DataStruct::parse(&input).unwrap();
        assert_eq!(struct_desc.fields.len(), 1);
        assert_eq!(struct_desc.fields[0].layout.offset.unwrap(), 2);
        assert_eq!(struct_desc.fields[0].layout.bit_field.as_ref().unwrap().bits, (1..2));
    }
}
