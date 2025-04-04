//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_manager::serialization::{Deserialize, InputStream, OutputStream, Seek as _, Serialize};

#[derive(Serialize, Deserialize)]
struct SimpleData {
    pub field_a: u32,
    pub field_b: u16,
}

#[derive(Serialize, Deserialize)]
struct OffsetData {
    #[layout(offset = 2)]
    pub field_a: u16,
}

#[derive(Serialize, Deserialize)]
struct BitField {
    #[layout(offset = 2, bit_field(u16, 15))]
    pub field_a: bool,
    #[layout(offset = 2, bit_field(u16, 0..15))]
    pub field_b: u16,
}

#[derive(Serialize, Deserialize)]
struct RoundedField {
    #[layout(round = 8)]
    pub field_a: u16,
}

#[derive(Serialize, Deserialize)]
#[layout(little_endian)]
struct ByteOrderData {
    #[layout(little_endian)]
    pub field_a: u16,
    #[layout(big_endian)]
    pub field_b: u16,
    // Inherit
    pub field_c: u16,
}

#[derive(Serialize, Deserialize)]
#[layout(round = 8)]
struct RoundedStructSingle {
    pub field_a: u16,
}

#[derive(Serialize, Deserialize)]
#[layout(round = 3)]
struct RoundedStructMultiple {
    pub field_a: u64,
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
enum SimpleEnum {
    A = 0x01,
    B = 0x02,
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
enum FallbackEnum {
    A = 0x01,
    #[fallback]
    Fallback = 0xFF,
}

#[test]
fn serialize_struct_simple() {
    let data = SimpleData { field_a: 0xABCDEF01, field_b: 0x2345 };

    let mut os = OutputStream::<u8>::new();
    data.serialize(&mut os).unwrap();

    assert_eq!(os.as_slice(), [0xAB_u8, 0xCD_u8, 0xEF_u8, 0x01_u8, 0x23_u8, 0x45_u8])
}

#[test]
fn serialize_struct_offset() {
    let data = OffsetData { field_a: 0x1234 };

    let mut os = OutputStream::<u8>::new();
    data.serialize(&mut os).unwrap();

    assert_eq!(os.as_slice(), [0x00_u8, 0x00_u8, 0x12_u8, 0x34_u8])
}

#[test]
fn serialize_struct_bitfield() {
    let data = BitField { field_a: true, field_b: 0x3FAB };

    let mut os = OutputStream::<u8>::new();
    data.serialize(&mut os).unwrap();

    assert_eq!(os.as_slice(), [0x00_u8, 0x00_u8, 0xBF_u8, 0xAB_u8])
}

#[test]
fn serialize_struct_rounded_field() {
    let data = RoundedField { field_a: 0x1234 };

    let mut os = OutputStream::<u8>::new();
    data.serialize(&mut os).unwrap();

    assert_eq!(os.as_slice(), [0x12_u8, 0x34_u8, 0x00_u8, 0x00_u8, 0x00_u8, 0x00_u8, 0x00_u8, 0x00_u8])
}

#[test]
fn serialize_struct_rounded_struct_single() {
    let data = RoundedStructSingle { field_a: 0x1234 };

    let mut os = OutputStream::<u8>::new();
    data.serialize(&mut os).unwrap();

    assert_eq!(os.as_slice(), [0x12_u8, 0x34_u8, 0x00_u8, 0x00_u8, 0x00_u8, 0x00_u8, 0x00_u8, 0x00_u8])
}

#[test]
fn serialize_struct_rounded_struct_multiple() {
    let data = RoundedStructMultiple { field_a: 0x12345678_91011126 };

    let mut os = OutputStream::<u8>::new();
    data.serialize(&mut os).unwrap();

    assert_eq!(os.as_slice(), [0x12_u8, 0x34_u8, 0x56_u8, 0x78_u8, 0x91_u8, 0x01_u8, 0x11_u8, 0x26_u8, 0x00])
}

const DESERIALIZE_DATA: [u8; 9] = [
    0x01_u8, 0x23_u8, 0x45_u8, 0x67_u8, 0x89_u8, 0xAB_u8, 0xCD_u8, 0xEF_u8, 0x00_u8,
];

#[test]
fn deserialize_struct_simple() {
    let mut is = InputStream::<u8>::new(&DESERIALIZE_DATA);
    let data = SimpleData::deserialize(&mut is).unwrap();

    assert_eq!(data.field_a, 0x01234567);
    assert_eq!(data.field_b, 0x89AB);
    assert_eq!(is.stream_position(), 6);
}

#[test]
fn deserialize_struct_offset() {
    let mut is = InputStream::<u8>::new(&DESERIALIZE_DATA);
    let data = OffsetData::deserialize(&mut is).unwrap();

    assert_eq!(data.field_a, 0x4567);
    assert_eq!(is.stream_position(), 4);
}

#[test]
fn deserialize_struct_bitfield() {
    let mut is = InputStream::<u8>::new(&DESERIALIZE_DATA);
    let data = BitField::deserialize(&mut is).unwrap();

    assert_eq!(data.field_a, false);
    assert_eq!(data.field_b, 0x4567);
    assert_eq!(is.stream_position(), 4);
}

#[test]
fn deserialize_struct_rounded_field() {
    let mut is = InputStream::<u8>::new(&DESERIALIZE_DATA);
    let data = RoundedField::deserialize(&mut is).unwrap();

    assert_eq!(data.field_a, 0x0123);
    assert_eq!(is.stream_position(), 8);
}

#[test]
fn deserialize_struct_rounded_struct_single() {
    let mut is = InputStream::<u8>::new(&DESERIALIZE_DATA);
    let data = RoundedStructSingle::deserialize(&mut is).unwrap();

    assert_eq!(data.field_a, 0x0123);
    assert_eq!(is.stream_position(), 8);
}

#[test]
fn deserialize_struct_rounded_struct_multiple() {
    let mut is = InputStream::<u8>::new(&DESERIALIZE_DATA);
    let data = RoundedStructMultiple::deserialize(&mut is).unwrap();

    assert_eq!(data.field_a, 0x0123456789ABCDEF);
    assert_eq!(is.stream_position(), 9);
}

#[test]
fn serialize_enum() {
    let mut os = OutputStream::<u8>::new();
    let input = SimpleEnum::B;
    input.serialize(&mut os).unwrap();
    let mut is = InputStream::from(os.take());
    let output = SimpleEnum::deserialize(&mut is).unwrap();
    assert_eq!(input, output);
}

#[test]
fn deserialize_enum_fallback() {
    let mut os = OutputStream::<u8>::new();
    0x34_u8.serialize(&mut os).unwrap();
    let mut is = InputStream::from(os.take());
    let output = FallbackEnum::deserialize(&mut is).unwrap();
    assert_eq!(FallbackEnum::Fallback, output);
}

#[test]
fn deserialize_struct_byte_order() {
    let mut is = InputStream::<u8>::new(&DESERIALIZE_DATA);
    let data = ByteOrderData::deserialize(&mut is).unwrap();
    assert_eq!(data.field_a, 0x2301);
    assert_eq!(data.field_b, 0x4567);
    assert_eq!(data.field_c, 0xAB89);
    assert_eq!(is.stream_position(), 6);
}

#[test]
fn serialize_struct_byte_order() {
    let mut os = OutputStream::<u8>::new();
    let data = ByteOrderData { field_a: 0x2301, field_b: 0x4567, field_c: 0xAB89 };
    data.serialize(&mut os).unwrap();
    assert_eq!(os.as_slice(), &DESERIALIZE_DATA[0..6]);
    assert_eq!(os.stream_position(), 6);
}
