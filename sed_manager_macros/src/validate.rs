use std::fmt::Display;
use std::ops::Range;

use crate::parse::EnumDesc;

use super::parse::{FieldDesc, Layout, StructDesc};

pub enum LayoutError {
    InvalidStructLayoutParam(String),
    ReversedFields((String, String)),
    OverlappingFields((String, String)),
    MultipleFallbacks,
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
            LayoutError::MultipleFallbacks => f.write_fmt(format_args!("enum must have at most one fallback variant")),
        }
    }
}

fn validate_struct_layout_params(layout: &Layout) -> Result<(), LayoutError> {
    if layout.offset.is_some() {
        return Err(LayoutError::InvalidStructLayoutParam(String::from("offset")));
    }
    if layout.bits.is_some() {
        return Err(LayoutError::InvalidStructLayoutParam(String::from("bits")));
    }
    if layout.fallback {
        return Err(LayoutError::InvalidStructLayoutParam(String::from("fallback")));
    }
    Ok(())
}

fn validate_field_layout_params(layout: &Layout) -> Result<(), LayoutError> {
    if layout.fallback {
        return Err(LayoutError::InvalidStructLayoutParam(String::from("fallback")));
    }
    Ok(())
}

fn validate_variant_layout_params(layout: &Layout) -> Result<(), LayoutError> {
    if layout.offset.is_some() {
        return Err(LayoutError::InvalidStructLayoutParam(String::from("offset")));
    }
    if layout.bits.is_some() {
        return Err(LayoutError::InvalidStructLayoutParam(String::from("bits")));
    }
    if layout.round.is_some() {
        return Err(LayoutError::InvalidStructLayoutParam(String::from("round")));
    }
    Ok(())
}

fn next_byte(bit: usize) -> usize {
    (bit + 7) / 8 * 8
}

fn field_locations(fields: &[FieldDesc]) -> Vec<Range<usize>> {
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

fn validate_location_reversal(descs: &[FieldDesc], locs: &[Range<usize>]) -> Result<(), LayoutError> {
    assert!(descs.len() == locs.len());
    for i in 1..descs.len() {
        if locs[i - 1].start > locs[i].start {
            return Err(LayoutError::ReversedFields((descs[i - 1].name.clone(), descs[i].name.clone())));
        }
    }
    Ok(())
}

fn validate_location_overlap(descs: &[FieldDesc], locs: &[Range<usize>]) -> Result<(), LayoutError> {
    assert!(descs.len() == locs.len());
    for i in 1..descs.len() {
        if locs[i - 1].end > locs[i].start {
            return Err(LayoutError::OverlappingFields((descs[i - 1].name.clone(), descs[i].name.clone())));
        }
    }
    Ok(())
}

pub fn validate_struct(desc: &StructDesc) -> Result<(), LayoutError> {
    validate_struct_layout_params(&desc.layout)?;
    for field in &desc.fields {
        validate_field_layout_params(&field.layout)?;
    }
    let locs = field_locations(&desc.fields);
    validate_location_reversal(&desc.fields, &locs)?; // Should do before overlap.
    validate_location_overlap(&desc.fields, &locs)?;
    Ok(())
}

pub fn validate_enum(desc: &EnumDesc) -> Result<(), LayoutError> {
    for field in &desc.variants {
        validate_variant_layout_params(&field.layout)?;
    }
    let num_fallbacks = desc.variants.iter().filter(|variant| variant.layout.fallback).count();
    if num_fallbacks > 1 {
        return Err(LayoutError::MultipleFallbacks);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::parse::Layout;
    use super::*;
    use quote::quote;

    #[test]
    fn validate_struct_layout_flagged_offset() {
        let layout = Layout { offset: Some(0), ..Default::default() };
        if let Err(LayoutError::InvalidStructLayoutParam(p)) = validate_struct_layout_params(&layout) {
            assert_eq!(p, "offset");
        } else {
            assert!(false);
        }
    }

    #[test]
    fn validate_struct_layout_flagged_bits() {
        let layout = Layout { bits: Some(0..1), ..Default::default() };
        if let Err(LayoutError::InvalidStructLayoutParam(p)) = validate_struct_layout_params(&layout) {
            assert_eq!(p, "bits");
        } else {
            assert!(false);
        }
    }

    #[test]
    fn field_locations_consecutive() {
        let fields = [
            FieldDesc { name: "0".into(), ty: quote! {}, layout: Layout { ..Default::default() } },
            FieldDesc { name: "1".into(), ty: quote! {}, layout: Layout { ..Default::default() } },
        ];
        let locations = field_locations(&fields);
        assert_eq!(locations[0], 0..8);
        assert_eq!(locations[1], 8..16);
    }

    #[test]
    fn field_locations_disjoint_bitfields() {
        let fields = [
            FieldDesc { name: "0".into(), ty: quote! {}, layout: Layout { bits: Some(1..2), ..Default::default() } },
            FieldDesc { name: "1".into(), ty: quote! {}, layout: Layout { bits: Some(3..4), ..Default::default() } },
        ];
        let locations = field_locations(&fields);
        assert_eq!(locations[0], 1..2);
        assert_eq!(locations[1], 11..12);
    }

    #[test]
    fn field_locations_joint_bitfields() {
        let fields = [
            FieldDesc {
                name: "0".into(),
                ty: quote! {},
                layout: Layout { bits: Some(1..2), offset: Some(0), ..Default::default() },
            },
            FieldDesc {
                name: "1".into(),
                ty: quote! {},
                layout: Layout { bits: Some(3..4), offset: Some(0), ..Default::default() },
            },
        ];
        let locations = field_locations(&fields);
        assert_eq!(locations[0], 1..2);
        assert_eq!(locations[1], 3..4);
    }

    #[test]
    fn field_locations_bitfield_follow() {
        let fields = [
            FieldDesc {
                name: "0".into(),
                ty: quote! {},
                layout: Layout { bits: Some(1..2), offset: Some(0), ..Default::default() },
            },
            FieldDesc { name: "1".into(), ty: quote! {}, layout: Layout { ..Default::default() } },
        ];
        let locations = field_locations(&fields);
        assert_eq!(locations[0], 1..2);
        assert_eq!(locations[1], 8..16);
    }

    #[test]
    fn field_locations_offset() {
        let fields = [
            FieldDesc { name: "0".into(), ty: quote! {}, layout: Layout { offset: Some(8), ..Default::default() } },
            FieldDesc { name: "1".into(), ty: quote! {}, layout: Layout { ..Default::default() } },
        ];
        let locations = field_locations(&fields);
        assert_eq!(locations[0], 64..72);
        assert_eq!(locations[1], 72..80);
    }

    #[test]
    fn field_locations_reversed() {
        let fields = [
            FieldDesc { name: "0".into(), ty: quote! {}, layout: Layout { offset: Some(8), ..Default::default() } },
            FieldDesc { name: "1".into(), ty: quote! {}, layout: Layout { offset: Some(6), ..Default::default() } },
            FieldDesc { name: "2".into(), ty: quote! {}, layout: Layout { ..Default::default() } },
        ];
        let locations = field_locations(&fields);
        assert_eq!(locations[0], 64..72);
        assert_eq!(locations[1], 48..56);
        assert_eq!(locations[2], 56..64);
    }

    #[test]
    fn validate_overlap_flagged() {
        let descs = [
            FieldDesc { name: "0".into(), ty: quote! {}, layout: Layout { ..Default::default() } },
            FieldDesc { name: "1".into(), ty: quote! {}, layout: Layout { ..Default::default() } },
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
            FieldDesc { name: "0".into(), ty: quote! {}, layout: Layout { ..Default::default() } },
            FieldDesc { name: "1".into(), ty: quote! {}, layout: Layout { ..Default::default() } },
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
