use sed_manager::messaging::value::{Bytes, Named, Value};
use sed_manager::spec::basic_types::Type;
use sed_manager_macros::AlternativeType;

#[derive(AlternativeType, PartialEq, Eq, Clone, Debug)]
enum AltSimple {
    OptionA(u8),
    OptionB(u16),
}

#[test]
fn to_value_normal() {
    let alt = AltSimple::OptionB(345);
    let value: Value = alt.into();
    let Ok(named) = &Named::try_from(&value) else {
        panic!("value is not a named");
    };
    let Ok(name) = &Bytes::try_from(&named.name) else {
        panic!("name is not bytes");
    };
    let Ok(value) = u16::try_from(&named.value) else {
        panic!("value is not u16");
    };
    assert_eq!(name.as_slice(), &<u16 as Type>::uid().as_u64().to_be_bytes()[4..]);
    assert_eq!(value, 345);
}

#[test]
fn try_from_value_opt_a() {
    let value = Value::from(Named {
        name: Vec::from(&<u8 as Type>::uid().as_u64().to_be_bytes()[4..]).into(),
        value: 234_u8.into(),
    });
    let Ok(alt) = AltSimple::try_from(value) else {
        panic!("conversion failed");
    };
    assert_eq!(alt, AltSimple::OptionA(234));
}

#[test]
fn try_from_value_opt_b() {
    let value = Value::from(Named {
        name: Vec::from(&<u16 as Type>::uid().as_u64().to_be_bytes()[4..]).into(),
        value: 345_u16.into(),
    });
    let Ok(alt) = AltSimple::try_from(value) else {
        panic!("conversion failed");
    };
    assert_eq!(alt, AltSimple::OptionB(345));
}

#[test]
fn try_from_value_unlisted_type() {
    let value = Value::from(Named {
        name: Vec::from(&<u32 as Type>::uid().as_u64().to_be_bytes()[4..]).into(),
        value: 345_u16.into(),
    });
    assert_eq!(AltSimple::try_from(value.clone()).unwrap_err(), value);
}

#[test]
fn try_from_value_invalid_type() {
    let value = Value::from(Named { name: 334467u64.into(), value: 345_u16.into() });
    assert_eq!(AltSimple::try_from(value.clone()).unwrap_err(), value);
}

#[test]
fn try_from_value_invalid_value() {
    let value = Value::from(Named {
        name: Vec::from(&<u16 as Type>::uid().as_u64().to_be_bytes()[4..]).into(),
        value: vec![14u8, 73u8, 33u8].into(),
    });
    assert_eq!(AltSimple::try_from(value.clone()).unwrap_err(), value);
}
