use core::array::from_fn;

use crate::messaging::value::{Named, Value};
use crate::variadics::with_variadic_pack;

use super::method::MethodStatus;

pub trait EncodeArgument {
    const OPTIONAL: bool;
    fn encode(self) -> Value;
    fn optional(&self) -> bool {
        Self::OPTIONAL
    }
}

impl<T: Into<Value>> EncodeArgument for T {
    const OPTIONAL: bool = false;
    fn encode(self) -> Value {
        self.into()
    }
}

impl<T: Into<Value>> EncodeArgument for Option<T> {
    const OPTIONAL: bool = true;
    fn encode(self) -> Value {
        match self {
            Some(content) => content.into(),
            None => Value::empty(),
        }
    }
}

pub trait TryDecodeArgument: Sized {
    const OPTIONAL: bool;
    type Error;
    fn try_decode(value: Value) -> Result<Self, Self::Error>;
}

impl<T: TryFrom<Value, Error = Value>> TryDecodeArgument for T {
    const OPTIONAL: bool = false;
    type Error = <T as TryFrom<Value>>::Error;
    fn try_decode(value: Value) -> Result<Self, Self::Error> {
        T::try_from(value)
    }
}

impl<T: TryFrom<Value>> TryDecodeArgument for Option<T> {
    const OPTIONAL: bool = true;
    type Error = <T as TryFrom<Value>>::Error;
    fn try_decode(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Empty => Ok(None),
            _ => Ok(Some(T::try_from(value)?)),
        }
    }
}

pub const fn is_valid<const N: usize>(optionals: &[bool; N]) -> bool {
    const fn is_valid_helper<const N: usize>(optionals: &[bool; N], idx: usize) -> bool {
        if idx < N {
            let item_valid = optionals[idx - 1] && optionals[idx] || !optionals[idx - 1];
            item_valid && is_valid_helper(optionals, idx + 1)
        } else {
            true
        }
    }
    is_valid_helper(optionals, 1)
}

pub fn get_labels<const N: usize>(optionals: &[bool; N]) -> [isize; N] {
    let mut prev: isize = -1;
    optionals.map(|pred| -> isize {
        let current = match pred {
            true => prev + 1,
            false => prev,
        };
        prev = current;
        current
    })
}

pub fn add_labels<const N: usize>(args: [Value; N], labels: [isize; N]) -> [Value; N] {
    let mut idx: usize = 0;
    args.map(|arg| -> Value {
        let new_value = if labels[idx] >= 0 && !arg.is_empty() {
            Value::from(Named { name: (labels[idx] as u16).into(), value: arg })
        } else {
            arg
        };
        idx += 1;
        new_value
    })
}

pub fn collapse<const N: usize>(args: [Value; N]) -> Vec<Value> {
    args.into_iter().filter(|value| !value.is_empty()).collect()
}

pub fn expand_args<const N: usize>(args: Vec<Value>, optionals: &[bool; N]) -> Result<[Value; N], MethodStatus> {
    fn get_index_value_pair(
        pos: usize,
        arg: Value,
        optional: bool,
        optional_offset: usize,
    ) -> Result<(usize, Value), MethodStatus> {
        if !optional {
            Ok((pos, arg))
        } else if let Ok(named) = Named::try_from(arg) {
            if let Ok(label) = u64::try_from(named.name) {
                Ok((label as usize + optional_offset, named.value))
            } else {
                Err(MethodStatus::InvalidParameter)
            }
        } else {
            Err(MethodStatus::InvalidParameter)
        }
    }

    let mut expanded = from_fn::<Value, N, _>(|_| Value::empty());
    let optional_offset = optionals.iter().filter(|x| !*x).count();
    for (source, arg) in args.into_iter().enumerate() {
        let (target, value) = get_index_value_pair(source, arg, optionals[source], optional_offset)?;
        if target < N {
            expanded[target] = value;
        } else {
            return Err(MethodStatus::InvalidParameter);
        }
    }
    Ok(expanded)
}

/// This function is a big hack, by C++ terms.
/// We have to turn an array of `Value`s into a tuple of concrete types,
/// but there is not `std::index_sequence` in Rust, so I have no idea
/// how to do it without this ugly mutability.
pub fn to_concrete<T: TryDecodeArgument>(
    values: &mut [Value],
    idx: &mut usize,
) -> Result<T, <T as TryDecodeArgument>::Error> {
    let value = core::mem::replace(&mut values[*idx], Value::empty());
    *idx += 1;
    T::try_decode(value)
}

pub fn to_value<T: EncodeArgument>(arg: T) -> Value {
    arg.encode()
}

pub trait IntoMethodArgs {
    fn into_method_args(self) -> Vec<Value>;
}

pub trait TryFromMethodArgs: Sized {
    type Error;
    fn try_from_method_args(args: Vec<Value>) -> Result<Self, Self::Error>;
}

pub trait UnwrapMethodArgs<Output> {
    type Error;
    fn unwrap_method_args(self) -> Result<Output, Self::Error>;
}

impl<Output> UnwrapMethodArgs<Output> for Vec<Value>
where
    Output: TryFromMethodArgs,
{
    type Error = <Output as TryFromMethodArgs>::Error;
    fn unwrap_method_args(self) -> Result<Output, Self::Error> {
        Output::try_from_method_args(self)
    }
}

macro_rules! impl_into_method_args{
    ($($types:ident),*) => {
        impl<$($types),*> IntoMethodArgs for ($($types),*,)
            where $($types: EncodeArgument),*
        {
            fn into_method_args(self) -> Vec<Value> {
                #[allow(non_snake_case)]
                let ($($types),*,) = self;
                assert!(is_valid(&[$($types.optional(),)*]), "optional parameters must be at the end");
                let predicates = [$($types.optional(),)*];
                let values = [$(to_value($types),)*];
                let labels = get_labels(&predicates);
                let labelled = add_labels(values, labels);
                collapse(labelled)
            }
        }
    };
}

macro_rules! impl_unwrap_method_args {
    ($($types:ident),*) => {
        impl<$($types),*> TryFromMethodArgs for ($($types),*,)
            where $($types: TryDecodeArgument),*
        {
            type Error = MethodStatus;
            fn try_from_method_args(args: Vec<Value>) -> Result<Self, Self::Error> {
                assert!(is_valid(&[$(<$types>::OPTIONAL,)*]), "optional parameters must be at the end");
                let mut idx: usize = 0;
                let mut expanded = expand_args(args, &[$(<$types>::OPTIONAL,)*])?;
                Ok(($(to_concrete::<$types>(&mut expanded, &mut idx).map_err(|_| MethodStatus::InvalidParameter)?,)*))
            }
        }
    };
}

with_variadic_pack!(impl_into_method_args);
with_variadic_pack!(impl_unwrap_method_args);

impl IntoMethodArgs for () {
    fn into_method_args(self) -> Vec<Value> {
        Vec::new()
    }
}

impl TryFromMethodArgs for () {
    type Error = MethodStatus;
    fn try_from_method_args(args: Vec<Value>) -> Result<Self, Self::Error> {
        if args.is_empty() {
            Ok(())
        } else {
            Err(MethodStatus::InvalidParameter)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_valid_empty() {
        assert!(is_valid(&[]));
    }

    #[test]
    fn is_valid_yes() {
        assert!(is_valid(&[false, false, false, true, true]));
    }

    #[test]
    fn is_valid_no() {
        assert!(!is_valid(&[false, false, true, false, true]));
    }

    #[test]
    fn get_labels_mixed() {
        let result = get_labels(&[false, false, false, true, true]);
        let expected = [-1, -1, -1, 0, 1];
        assert_eq!(result, expected);
    }

    #[test]
    fn get_labels_only_optionals() {
        let result = get_labels(&[true, true]);
        let expected = [0, 1];
        assert_eq!(result, expected);
    }

    #[test]
    fn add_labels_mixed() {
        let result = add_labels([10_u32.into(), Value::empty(), 12_u32.into()], [-1, 0, 1]);
        let expected = [
            10_u32.into(),
            Value::empty(),
            Named { name: 1_u16.into(), value: 12_u32.into() }.into(),
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn expand_args_no_optionals() -> Result<(), MethodStatus> {
        let args = vec![Value::from(0_u32), Value::from(1_u32)];
        let result = expand_args(args, &[false, false])?;
        let expected = [Value::from(0_u32), Value::from(1_u32)];
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn expand_args_optionals() -> Result<(), MethodStatus> {
        let args = vec![
            Value::from(Named { name: 1_u16.into(), value: 0_u32.into() }),
            Value::from(Named { name: 3_u16.into(), value: 1_u32.into() }),
        ];
        let result = expand_args(args, &[true, true, true, true, true])?;
        let expected = [
            Value::empty(),
            Value::from(0_u32),
            Value::empty(),
            Value::from(1_u32),
            Value::empty(),
        ];
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn encode_args_mixed() {
        let result = (0_u32, 1_u32, Option::<u32>::None, Some(3_u32), Option::<u32>::None).into_method_args();
        let expected = [
            Value::from(0_u32),
            Value::from(1_u32),
            Value::from(Named { name: 1_u16.into(), value: 3_u32.into() }),
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn unwrap_method_args_mixed() -> Result<(), MethodStatus> {
        let args = vec![
            Value::from(0_u32),
            Value::from(1_u32),
            Value::from(Named { name: 1_u16.into(), value: 3_u32.into() }),
        ];
        let result: (u32, u32, Option<u32>, Option<u32>, Option<u32>) = args.unwrap_method_args()?;
        let expected = (0_u32, 1_u32, None, Some(3_u32), None);
        assert_eq!(result, expected);
        Ok(())
    }
}
