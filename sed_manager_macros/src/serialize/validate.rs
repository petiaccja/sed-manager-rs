use std::fmt::Display;
use std::ops::Range;

use crate::parse::data_struct::{DataField, DataStruct, LayoutAttr};
use crate::parse::numeric_enum::NumericEnum;

pub enum LayoutError {
    InvalidStructLayoutParam(String),
    ReversedFields((String, String)),
    OverlappingFields((String, String)),
}

impl Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
        }
    }
}

fn validate_struct_layout_params(layout: &LayoutAttr) -> Result<(), LayoutError> {
    if layout.offset.is_some() {
        return Err(LayoutError::InvalidStructLayoutParam(String::from("offset")));
    }
    if layout.bits.is_some() {
        return Err(LayoutError::InvalidStructLayoutParam(String::from("bits")));
    }
    Ok(())
}

fn validate_field_layout_params(_layout: &LayoutAttr) -> Result<(), LayoutError> {
    Ok(())
}

fn next_byte(bit: usize) -> usize {
    (bit + 7) / 8 * 8
}

fn field_locations(fields: &[DataField]) -> Vec<Range<usize>> {
    let mut locations = Vec::<Range<usize>>::new();
    for field in fields {
        let base = match field.layout.offset {
            Some(offset) => 8 * offset as usize,
            None => match locations.last() {
                Some(last) => next_byte(last.end),
                None => 0,
            },
        };
        let span = match &field.layout.bits {
            Some(bits) => (base + bits.start)..(base + bits.end),
            None => base..(base + 8), // Estimate size as 1 byte. May add a const SIZE to Serialize trait.
        };
        let rounded = match field.layout.round {
            Some(round) => {
                let len = span.end - base;
                let rounded = (len + 8 * round - 1) / (8 * round) * (8 * round);
                (span.start)..(base + rounded)
            }
            None => span,
        };
        locations.push(rounded);
    }
    locations
}

fn validate_location_reversal(descs: &[DataField], locs: &[Range<usize>]) -> Result<(), LayoutError> {
    assert!(descs.len() == locs.len());
    for i in 1..descs.len() {
        if locs[i - 1].start > locs[i].start {
            return Err(LayoutError::ReversedFields((descs[i - 1].name.to_string(), descs[i].name.to_string())));
        }
    }
    Ok(())
}

fn validate_location_overlap(descs: &[DataField], locs: &[Range<usize>]) -> Result<(), LayoutError> {
    assert!(descs.len() == locs.len());
    for i in 1..descs.len() {
        if locs[i - 1].end > locs[i].start {
            return Err(LayoutError::OverlappingFields((descs[i - 1].name.to_string(), descs[i].name.to_string())));
        }
    }
    Ok(())
}

pub fn validate_struct(desc: &DataStruct) -> Result<(), LayoutError> {
    validate_struct_layout_params(&desc.layout)?;
    for field in &desc.fields {
        validate_field_layout_params(&field.layout)?;
    }
    let locs = field_locations(&desc.fields);
    validate_location_reversal(&desc.fields, &locs)?; // Should do before overlap.
    validate_location_overlap(&desc.fields, &locs)?;
    Ok(())
}

pub fn validate_enum(_desc: &NumericEnum) -> Result<(), LayoutError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::Span;
    use quote::quote;

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
        let layout = LayoutAttr { bits: Some(0..1), ..Default::default() };
        if let Err(LayoutError::InvalidStructLayoutParam(p)) = validate_struct_layout_params(&layout) {
            assert_eq!(p, "bits");
        } else {
            assert!(false);
        }
    }

    #[test]
    fn field_locations_consecutive() {
        let fields = [
            DataField {
                name: syn::Ident::new("0", Span::call_site()),
                ty: syn::Type::Verbatim(quote! {u8}),
                layout: LayoutAttr { ..Default::default() },
            },
            DataField {
                name: syn::Ident::new("1", Span::call_site()),
                ty: syn::Type::Verbatim(quote! {u8}),
                layout: LayoutAttr { ..Default::default() },
            },
        ];
        let locations = field_locations(&fields);
        assert_eq!(locations[0], 0..8);
        assert_eq!(locations[1], 8..16);
    }

    #[test]
    fn field_locations_disjoint_bitfields() {
        let fields = [
            DataField {
                name: syn::Ident::new("0", Span::call_site()),
                ty: syn::Type::Verbatim(quote! {u8}),
                layout: LayoutAttr { bits: Some(1..2), ..Default::default() },
            },
            DataField {
                name: syn::Ident::new("1", Span::call_site()),
                ty: syn::Type::Verbatim(quote! {u8}),
                layout: LayoutAttr { bits: Some(3..4), ..Default::default() },
            },
        ];
        let locations = field_locations(&fields);
        assert_eq!(locations[0], 1..2);
        assert_eq!(locations[1], 11..12);
    }

    #[test]
    fn field_locations_joint_bitfields() {
        let fields = [
            DataField {
                name: syn::Ident::new("0", Span::call_site()),
                ty: syn::Type::Verbatim(quote! {u8}),
                layout: LayoutAttr { bits: Some(1..2), offset: Some(0), ..Default::default() },
            },
            DataField {
                name: syn::Ident::new("1", Span::call_site()),
                ty: syn::Type::Verbatim(quote! {u8}),
                layout: LayoutAttr { bits: Some(3..4), offset: Some(0), ..Default::default() },
            },
        ];
        let locations = field_locations(&fields);
        assert_eq!(locations[0], 1..2);
        assert_eq!(locations[1], 3..4);
    }

    #[test]
    fn field_locations_bitfield_follow() {
        let fields = [
            DataField {
                name: syn::Ident::new("0", Span::call_site()),
                ty: syn::Type::Verbatim(quote! {u8}),
                layout: LayoutAttr { bits: Some(1..2), offset: Some(0), ..Default::default() },
            },
            DataField {
                name: syn::Ident::new("1", Span::call_site()),
                ty: syn::Type::Verbatim(quote! {u8}),
                layout: LayoutAttr { ..Default::default() },
            },
        ];
        let locations = field_locations(&fields);
        assert_eq!(locations[0], 1..2);
        assert_eq!(locations[1], 8..16);
    }

    #[test]
    fn field_locations_offset() {
        let fields = [
            DataField {
                name: syn::Ident::new("0", Span::call_site()),
                ty: syn::Type::Verbatim(quote! {u8}),
                layout: LayoutAttr { offset: Some(8), ..Default::default() },
            },
            DataField {
                name: syn::Ident::new("1", Span::call_site()),
                ty: syn::Type::Verbatim(quote! {u8}),
                layout: LayoutAttr { ..Default::default() },
            },
        ];
        let locations = field_locations(&fields);
        assert_eq!(locations[0], 64..72);
        assert_eq!(locations[1], 72..80);
    }

    #[test]
    fn field_locations_reversed() {
        let fields = [
            DataField {
                name: syn::Ident::new("0", Span::call_site()),
                ty: syn::Type::Verbatim(quote! {u8}),
                layout: LayoutAttr { offset: Some(8), ..Default::default() },
            },
            DataField {
                name: syn::Ident::new("1", Span::call_site()),
                ty: syn::Type::Verbatim(quote! {u8}),
                layout: LayoutAttr { offset: Some(6), ..Default::default() },
            },
            DataField {
                name: syn::Ident::new("2", Span::call_site()),
                ty: syn::Type::Verbatim(quote! {u8}),
                layout: LayoutAttr { ..Default::default() },
            },
        ];
        let locations = field_locations(&fields);
        assert_eq!(locations[0], 64..72);
        assert_eq!(locations[1], 48..56);
        assert_eq!(locations[2], 56..64);
    }

    #[test]
    fn validate_overlap_flagged() {
        let descs = [
            DataField {
                name: syn::Ident::new("1", Span::call_site()).into(),
                ty: syn::Type::Verbatim(quote! {u8}),
                layout: LayoutAttr { ..Default::default() },
            },
            DataField {
                name: syn::Ident::new("1", Span::call_site()).into(),
                ty: syn::Type::Verbatim(quote! {u8}),
                layout: LayoutAttr { ..Default::default() },
            },
        ];
        let locs = [0..16, 8..24];
        if let Err(LayoutError::OverlappingFields((a, b))) = validate_location_overlap(&descs, &locs) {
            assert_eq!(a, "0");
            assert_eq!(b, "1");
        } else {
            assert!(false);
        }
    }

    #[test]
    fn validate_reversed_flagged() {
        let descs = [
            DataField {
                name: syn::Ident::new("0", Span::call_site()),
                ty: syn::Type::Verbatim(quote! {u8}),
                layout: LayoutAttr { ..Default::default() },
            },
            DataField {
                name: syn::Ident::new("1", Span::call_site()),
                ty: syn::Type::Verbatim(quote! {u8}),
                layout: LayoutAttr { ..Default::default() },
            },
        ];
        let locs = [32..40, 0..8];
        if let Err(LayoutError::ReversedFields((a, b))) = validate_location_reversal(&descs, &locs) {
            assert_eq!(a, "0");
            assert_eq!(b, "1");
        } else {
            assert!(false);
        }
    }
}
