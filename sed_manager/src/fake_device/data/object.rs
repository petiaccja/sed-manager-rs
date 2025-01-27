use crate::messaging::uid::UID;
use crate::messaging::value::Value;
use crate::variadics::with_variadic_pack;

use super::cell::Cell;
use super::cell::CellData;

pub trait Object {
    fn uid(&self) -> UID;
    fn get(&self, cell: usize) -> Value;
    fn try_set(&mut self, cell: usize, value: Value) -> Result<(), Value>;
    fn is_empty(&self, cell: usize) -> bool;
}

pub struct ObjectData<T: ObjectTuple>(T);

pub trait ObjectTuple {
    fn uid(&self) -> UID;
    fn at(&self, cell: usize) -> &dyn Cell;
    fn at_mut(&mut self, cell: usize) -> &mut dyn Cell;
    fn len(&self) -> usize;
}

impl<T: ObjectTuple> Object for ObjectData<T> {
    fn uid(&self) -> UID {
        self.0.uid()
    }

    fn get(&self, cell: usize) -> Value {
        self.0.at(cell).get()
    }

    fn try_set(&mut self, cell: usize, value: Value) -> Result<(), Value> {
        // UID cell is immutable.
        if cell != 0 {
            self.0.at_mut(cell).try_set(value)
        } else {
            Err(value)
        }
    }

    fn is_empty(&self, cell: usize) -> bool {
        self.0.at(cell).is_empty()
    }
}

macro_rules! impl_object_tuple {
    ($($types:ident),*) => {
        impl<$($types),*> ObjectTuple for (CellData<UID>, $($types),*,)
        where $($types: Cell),*,
        {
            fn uid(&self) -> UID {
                CellData::<UID>::get(&self.0).unwrap_or(&UID::null()).clone()
            }

            fn at(&self, cell: usize) -> &dyn Cell {
                #[allow(non_snake_case)]
                let (uid, $($types),*,) = self;
                let cells = [uid, $($types as &dyn Cell),*,];
                cells[cell]
            }

            fn at_mut(&mut self, cell: usize) -> &mut dyn Cell {
                #[allow(non_snake_case)]
                let (uid, $($types),*,) = self;
                let cells = [uid, $($types as &mut dyn Cell),*,];
                cells[cell]
            }

            fn len(&self) -> usize {
                fn one<Q>() -> usize { 1 }
                (1 $(+ one::<$types>())*)
            }
        }
    };
}

with_variadic_pack!(impl_object_tuple);

impl<T: ObjectTuple> std::hash::Hash for ObjectData<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.uid().value());
    }
}

impl<T: ObjectTuple> std::cmp::PartialEq for ObjectData<T> {
    fn eq(&self, other: &Self) -> bool {
        self.uid() == other.uid()
    }
}
