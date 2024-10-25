use sed_manager::serialization::{OutputStream, Serialize};

#[derive(Serialize)]
struct SimpleData {
    pub field_a: u32,
    pub field_b: u16,
}

#[derive(Serialize)]
struct OffsetData {
    #[layout(offset = 2)]
    pub field_a: u16,
}

#[derive(Serialize)]
struct BitField {
    #[layout(offset = 2, bits = 0..1)]
    pub field_a: bool,
    #[layout(offset = 2, bits = 1..16)]
    pub field_b: u16,
}

#[derive(Serialize)]
struct RoundedField {
    #[layout(round = 8)]
    pub field_a: u16,
}

#[derive(Serialize)]
#[layout(round = 8)]
struct RoundedStructShort {
    pub field_a: u16,
}

#[derive(Serialize)]
#[layout(round = 3)]
struct RoundedStructLong {
    pub field_a: u64,
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
fn serialize_struct_rounded_struct_short() {
    let data = RoundedStructShort { field_a: 0x1234 };

    let mut os = OutputStream::<u8>::new();
    data.serialize(&mut os).unwrap();

    assert_eq!(os.as_slice(), [0x12_u8, 0x34_u8, 0x00_u8, 0x00_u8, 0x00_u8, 0x00_u8, 0x00_u8, 0x00_u8])
}

#[test]
fn serialize_struct_rounded_struct_long() {
    let data = RoundedStructLong { field_a: 0x12345678_91011126 };

    let mut os = OutputStream::<u8>::new();
    data.serialize(&mut os).unwrap();

    assert_eq!(os.as_slice(), [0x12_u8, 0x34_u8, 0x56_u8, 0x78_u8, 0x91_u8, 0x01_u8, 0x11_u8, 0x26_u8, 0x00])
}
