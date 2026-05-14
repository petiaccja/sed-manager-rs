//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use quote::ToTokens;

use crate::parse::data_struct::{DataField, DataStruct, LayoutAttr};
use crate::parse::numeric_enum::NumericEnum;
use core::fmt::Display;

#[derive(Debug, PartialEq, Eq)]
pub enum LayoutError {
    InvalidStructLayoutParam(String),
    ReversedFields((String, String)),
    OverlappingFields((String, String)),
    BitFieldTypeMismatch(String),
}

impl Display for LayoutError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LayoutError::InvalidStructLayoutParam(param) => {
                f.write_fmt(format_args!("parameter `{}` is not allowed on structs", param))
            }
            LayoutError::ReversedFields((a, b)) => f.write_fmt(format_args!(
                "fields `{}` and `{}` must follow the same order in serialization as their declaration",
                a, b
            )),
            LayoutError::OverlappingFields((a, b)) => {
                f.write_fmt(format_args!("fields `{}` and `{}` cannot overlap", a, b))
            }
            LayoutError::BitFieldTypeMismatch(field) => {
                write!(f, "bit field of `{field}` contains fields with different types")
            }
        }
    }
}

fn validate_struct_layout_params(layout: &LayoutAttr) -> Result<(), LayoutError> {
    if layout.offset.is_some() {
        return Err(LayoutError::InvalidStructLayoutParam(String::from("offset")));
    }
    if layout.bit_field.is_some() {
        return Err(LayoutError::InvalidStructLayoutParam(String::from("bit_field")));
    }
    Ok(())
}

fn validate_field_layout_params(_layout: &LayoutAttr) -> Result<(), LayoutError> {
    Ok(())
}

fn separate_groups(fields: &[DataField]) -> impl Iterator<Item = &[DataField]> {
    fields.chunk_by(|first, second| first.layout.offset.is_some() && first.layout.offset == second.layout.offset)
}

fn are_offsets_increasing<'a>(groups: impl Iterator<Item = &'a [DataField]>) -> Result<(), LayoutError> {
    let is_sorted = groups.filter_map(|group| group.first().map(|first| first.layout.offset).flatten()).is_sorted();
    if is_sorted {
        Ok(())
    } else {
        Err(LayoutError::ReversedFields(("<unknown>".into(), "<unknown>".into())))
    }
}

fn is_group_bit_field(group: &[DataField]) -> bool {
    group.iter().any(|field| field.layout.bit_field.is_some())
}

fn is_group_valid(group: &[DataField]) -> Result<(), LayoutError> {
    if is_group_bit_field(group) {
        let mut bit_fields = group.iter().filter_map(|field| field.layout.bit_field.as_ref()).collect::<Vec<_>>();
        if bit_fields.len() != group.len() {
            let first_non_bit_field = group.iter().find(|field| field.layout.bit_field.is_none()).unwrap();
            let first_bit_field = group.iter().find(|field| field.layout.bit_field.is_some()).unwrap();
            return Err(LayoutError::OverlappingFields((
                first_bit_field.name.to_string(),
                first_non_bit_field.name.to_string(),
            )));
        }

        let same_types = bit_fields.iter().all(|&bit_field| {
            bit_field.ty.to_token_stream().to_string() == bit_fields[0].ty.to_token_stream().to_string()
        });
        if !same_types {
            return Err(LayoutError::BitFieldTypeMismatch(group[0].name.to_string()));
        }

        bit_fields.sort_by_key(|bit_field| bit_field.bits.start);
        for idx in 1..bit_fields.len() {
            let prev_bf = &bit_fields[idx - 1].bits;
            let cur_bf = &bit_fields[idx].bits;
            if prev_bf.end > cur_bf.start {
                let prev_field = group
                    .iter()
                    .find(|field| field.layout.bit_field.as_ref().is_some_and(|bf| &bf.bits == prev_bf))
                    .unwrap();
                let cur_field = group
                    .iter()
                    .find(|field| field.layout.bit_field.as_ref().is_some_and(|bf| &bf.bits == cur_bf))
                    .unwrap();
                return Err(LayoutError::OverlappingFields((prev_field.name.to_string(), cur_field.name.to_string())));
            }
        }

        Ok(())
    } else {
        if group.len() >= 2 {
            Err(LayoutError::OverlappingFields((group[0].name.to_string(), group[1].name.to_string())))
        } else {
            Ok(())
        }
    }
}

pub fn validate_struct(desc: &DataStruct) -> Result<(), LayoutError> {
    validate_struct_layout_params(&desc.layout)?;
    for field in &desc.fields {
        validate_field_layout_params(&field.layout)?;
    }
    are_offsets_increasing(separate_groups(&desc.fields))?;
    for group in separate_groups(&desc.fields) {
        is_group_valid(group)?;
    }
    Ok(())
}

pub fn validate_enum(_desc: &NumericEnum) -> Result<(), LayoutError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::parse::data_struct::BitField;

    use super::*;
    use proc_macro2::Span;
    use quote::quote;

    macro_rules! name {
        ($name:expr) => {
            syn::Ident::new($name, Span::call_site())
        };
    }

    macro_rules! ty {
        ($ty:ty) => {
            syn::Type::Verbatim(quote! {$ty})
        };
    }

    #[test]
    fn validate_struct_layout_flagged_offset() {
        let layout = LayoutAttr { offset: Some(0), ..Default::default() };
        if let Err(LayoutError::InvalidStructLayoutParam(p)) = validate_struct_layout_params(&layout) {
            assert_eq!(p, "offset");
        } else {
            assert!(false);
        }
    }

    #[test]
    fn validate_struct_layout_flagged_bits() {
        let layout = LayoutAttr { bit_field: Some(BitField { bits: 1..2, ty: ty!(u8) }), ..Default::default() };
        if let Err(LayoutError::InvalidStructLayoutParam(p)) = validate_struct_layout_params(&layout) {
            assert_eq!(p, "bit_field");
        } else {
            assert!(false);
        }
    }

    #[test]
    fn good_default() {
        let desc = DataStruct {
            fields: vec![
                DataField { name: name!("_0"), ty: ty!(u8), layout: LayoutAttr { ..Default::default() } },
                DataField { name: name!("_1"), ty: ty!(u8), layout: LayoutAttr { ..Default::default() } },
            ],
            layout: LayoutAttr::default(),
            name: name!("S"),
        };
        assert_eq!(validate_struct(&desc), Ok(()));
    }

    #[test]
    fn good_offset() {
        let desc = DataStruct {
            fields: vec![
                DataField {
                    name: name!("_0"),
                    ty: ty!(u8),
                    layout: LayoutAttr { offset: Some(0), ..Default::default() },
                },
                DataField {
                    name: name!("_1"),
                    ty: ty!(u8),
                    layout: LayoutAttr { offset: Some(1), ..Default::default() },
                },
            ],
            layout: LayoutAttr::default(),
            name: name!("S"),
        };
        assert_eq!(validate_struct(&desc), Ok(()));
    }

    #[test]
    fn good_bit_field() {
        let desc = DataStruct {
            fields: vec![
                DataField {
                    name: name!("_0"),
                    ty: ty!(u8),
                    layout: LayoutAttr {
                        offset: Some(0),
                        bit_field: Some(BitField { bits: 1..2, ty: ty!(u8) }),
                        ..Default::default()
                    },
                },
                DataField {
                    name: name!("_1"),
                    ty: ty!(u8),
                    layout: LayoutAttr {
                        offset: Some(0),
                        bit_field: Some(BitField { bits: 2..3, ty: ty!(u8) }),
                        ..Default::default()
                    },
                },
            ],
            layout: LayoutAttr::default(),
            name: name!("S"),
        };
        assert_eq!(validate_struct(&desc), Ok(()));
    }

    #[test]
    fn offset_overlapping() {
        let desc = DataStruct {
            fields: vec![
                DataField {
                    name: name!("_0"),
                    ty: ty!(u8),
                    layout: LayoutAttr { offset: Some(2), ..Default::default() },
                },
                DataField {
                    name: name!("_1"),
                    ty: ty!(u8),
                    layout: LayoutAttr { offset: Some(2), ..Default::default() },
                },
            ],
            layout: LayoutAttr::default(),
            name: name!("S"),
        };
        let expected = Err(LayoutError::OverlappingFields(("_0".into(), "_1".into())));
        assert_eq!(validate_struct(&desc), expected);
    }

    #[test]
    fn bit_field_overlapping() {
        let desc = DataStruct {
            fields: vec![
                DataField {
                    name: name!("_0"),
                    ty: ty!(u8),
                    layout: LayoutAttr {
                        offset: Some(0),
                        bit_field: Some(BitField { bits: 1..4, ty: ty!(u8) }),
                        ..Default::default()
                    },
                },
                DataField {
                    name: name!("_1"),
                    ty: ty!(u8),
                    layout: LayoutAttr {
                        offset: Some(0),
                        bit_field: Some(BitField { bits: 3..8, ty: ty!(u8) }),
                        ..Default::default()
                    },
                },
            ],
            layout: LayoutAttr::default(),
            name: name!("S"),
        };
        let expected = Err(LayoutError::OverlappingFields(("_0".into(), "_1".into())));
        assert_eq!(validate_struct(&desc), expected);
    }
}
