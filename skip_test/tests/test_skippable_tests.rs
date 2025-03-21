//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use skip_test::{may_skip, skip, skip_or_unwrap};

#[test]
#[may_skip]
fn no_skip_attr_second() {}

#[may_skip]
#[test]
fn no_skip_attr_first() {}

#[test]
#[may_skip]
#[should_panic]
fn skip_should_panic_first() {
    assert_eq!(1, 0);
}

#[test]
#[should_panic]
#[may_skip]
fn skip_should_panic_second() {
    assert_eq!(1, 0);
}

#[test]
#[may_skip]
fn skip_unconditional() {
    skip!();
    #[allow(unreachable_code)]
    {
        panic!("hard failure");
    }
}

#[test]
#[may_skip]
fn skip_unwrap_err() {
    skip_or_unwrap!(Err(()));
    panic!("hard failure");
}

#[test]
#[may_skip]
fn skip_unwrap_none() {
    skip_or_unwrap!(None);
    panic!("hard failure");
}

#[test]
#[may_skip]
fn skip_unwrap_ok() {
    let result: Result<u8, ()> = Ok(1);
    let value = skip_or_unwrap!(result);
    assert_eq!(value, 1);
}

#[test]
#[may_skip]
fn skip_unwrap_some() {
    let value = skip_or_unwrap!(Some(1u8));
    assert_eq!(value, 1);
}

#[test]
#[may_skip]
fn skip_with_result() -> Result<(), ()> {
    skip!();
    #[allow(unreachable_code)]
    {
        panic!("hard failure");
    }
}
