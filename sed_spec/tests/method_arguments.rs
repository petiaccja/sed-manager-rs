use rstest::rstest;

use sed_packet::token::{FromTokens, ToTokens};
use sed_spec_macros::{DetokenizeStruct, TokenizeStruct};

#[derive(Debug, Clone, PartialEq, Eq, DetokenizeStruct, TokenizeStruct)]
struct TestArgs {
    a: u8,
    b: u16,
    c: Option<u8>,
    d: Option<u16>,
}

#[rstest]
#[case(TestArgs{a: 0xA, b: 0xB, c: None, d: None }, &[0xF0, 0xA, 0xB, 0xF1])]
#[case(TestArgs{a: 0xA, b: 0xB, c: Some(0xC), d: None }, &[0xF0, 0xA, 0xB, 0xF2, 0x0, 0xC, 0xF3, 0xF1])]
#[case(TestArgs{a: 0xA, b: 0xB, c: Some(0xC), d: Some(0xD) }, &[0xF0, 0xA, 0xB, 0xF2, 0x0, 0xC, 0xF3, 0xF2, 0x1, 0xD, 0xF3, 0xF1])]
#[case(TestArgs{a: 0xA, b: 0xB, c: None, d: Some(0xD) }, &[0xF0, 0xA, 0xB, 0xF2, 0x1, 0xD, 0xF3, 0xF1])]
fn tokenization(#[case] value: TestArgs, #[case] bytes: &[u8]) {
    assert_eq!(value.to_tokens().unwrap(), bytes);
    assert_eq!(TestArgs::from_tokens(bytes).unwrap(), value);
}

#[test]
#[should_panic]
fn detokenization_missing_mandatory() {
    let bytes = &[0xF0, 0xA, 0xF1];
    TestArgs::from_tokens(bytes).unwrap();
}

#[test]
#[should_panic]
fn detokenization_bad_optional() {
    let bytes = &[0xF0, 0xA, 0xB, 0xF2, 0x6, 0xD, 0xF3, 0xF1];
    TestArgs::from_tokens(bytes).unwrap();
}
