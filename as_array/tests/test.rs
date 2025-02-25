use as_array::AsArray;

#[derive(AsArray)]
#[as_array_traits(core::any::Any(as_any_array))]
struct TupleStruct(u32, bool);

#[derive(AsArray)]
#[as_array_traits(core::any::Any(as_any_array), core::any::Any(as_any_2_array))]
struct FieldStruct {
    f1: u32,
    f2: bool,
}

#[derive(AsArray)]
#[as_array_traits(core::any::Any)]
struct DefaultName {
    f1: u32,
}

#[test]
fn tuple_struct_ref() {
    let value = TupleStruct(0, true);
    let array = value.as_any_array();
    assert_eq!(array[0].downcast_ref::<u32>().unwrap(), &0);
    assert_eq!(array[1].downcast_ref::<bool>().unwrap(), &true);
}

#[test]
fn tuple_struct_mut() {
    let mut value = TupleStruct(0, true);
    let array = value.as_any_array_mut();
    assert_eq!(array[0].downcast_mut::<u32>().unwrap(), &0);
    assert_eq!(array[1].downcast_mut::<bool>().unwrap(), &true);
}

#[test]
fn field_struct_ref() {
    let value = FieldStruct { f1: 0, f2: true };
    let array = value.as_any_array();
    let array_2 = value.as_any_2_array();
    assert_eq!(array[0].downcast_ref::<u32>().unwrap(), &0);
    assert_eq!(array[1].downcast_ref::<bool>().unwrap(), &true);
    assert_eq!(array_2[0].downcast_ref::<u32>().unwrap(), &0);
    assert_eq!(array_2[1].downcast_ref::<bool>().unwrap(), &true);
}

#[test]
fn field_struct_mut() {
    let mut value = FieldStruct { f1: 0, f2: true };
    {
        let array = value.as_any_array_mut();
        assert_eq!(array[0].downcast_mut::<u32>().unwrap(), &0);
        assert_eq!(array[1].downcast_mut::<bool>().unwrap(), &true);
    }
    {
        let array_2 = value.as_any_2_array_mut();
        assert_eq!(array_2[0].downcast_mut::<u32>().unwrap(), &0);
        assert_eq!(array_2[1].downcast_mut::<bool>().unwrap(), &true);
    }
}

#[test]
fn default_name_ref() {
    let value = DefaultName { f1: 0 };
    let array = value.as_array();
    assert_eq!(array[0].downcast_ref::<u32>().unwrap(), &0);
}

#[test]
fn default_name_mut() {
    let mut value = DefaultName { f1: 0 };
    let array = value.as_array_mut();
    assert_eq!(array[0].downcast_mut::<u32>().unwrap(), &0);
}
