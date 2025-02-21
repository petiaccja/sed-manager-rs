//! Poor man's std::apply.

use super::variadics::with_variadic_pack;

pub trait CallWithTuple<Output, Tuple> {
    #[allow(unused)]
    fn call_with_tuple(&self, args: Tuple) -> Output;
}

pub trait CallSelfWithTuple<This, Output, Tuple> {
    fn call_self_with_tuple(&self, this: This, args: Tuple) -> Output;
}

macro_rules! impl_call_with_tuple {
    ($($types:ident),*) => {
        impl<Function, Output, $($types),*> CallWithTuple<Output, ($($types),*,)> for Function
        where
            Self: Fn($($types),*) -> Output,
        {
            fn call_with_tuple(&self, args: ($($types),*,)) -> Output {
                #[allow(non_snake_case)]
                let ($($types),*,) = args;
                self($($types),*)
            }
        }
    };
}

macro_rules! impl_call_self_with_tuple {
    ($($types:ident),*) => {
        impl<Function, This, Output, $($types),*> CallSelfWithTuple<This, Output, ($($types),*,)> for Function
        where
            Self: Fn(This, $($types),*) -> Output,
        {
            fn call_self_with_tuple(&self, this: This, args: ($($types),*,)) -> Output {
                #[allow(non_snake_case)]
                let ($($types),*,) = args;
                self(this, $($types),*)
            }
        }
    };
}

with_variadic_pack!(impl_call_with_tuple);
with_variadic_pack!(impl_call_self_with_tuple);

#[cfg(test)]
mod tests {
    use super::*;

    fn add(a: i32, b: i32) -> i32 {
        a + b
    }

    fn add_ref(a: &mut i32, b: i32) {
        *a += b
    }

    struct Object {}

    impl Object {
        fn add(&self, a: i32, b: i32) -> i32 {
            a + b
        }

        fn add_mut_self(&mut self, a: &mut i32, b: i32) {
            *a += b
        }
    }

    #[test]
    fn test_fn() {
        assert_eq!(add.call_with_tuple((3, 4)), 7);
    }

    #[test]
    fn test_fn_ref() {
        let mut a = 3;
        add_ref.call_with_tuple((&mut a, 4));
        assert_eq!(a, 7);
    }

    #[test]
    fn test_self_fn() {
        let object = Object {};
        assert_eq!(Object::add.call_self_with_tuple(&object, (3, 4)), 7);
    }

    #[test]
    fn test_self_fn_mut_self() {
        let mut object = Object {};
        let mut a = 3;
        Object::add_mut_self.call_self_with_tuple(&mut object, (&mut a, 4));
        assert_eq!(a, 7);
    }
}
