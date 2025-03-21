//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use super::{annotate_field, stream::Seek, Deserialize, Error, InputStream, OutputStream, Serialize};
use core::{ops::Deref, ops::DerefMut};

/// A vector of `T` with special a serialization format.
///
/// The elements are serialized consecutively, but the number of elements
/// is not stored in any way. When deserializing, all reimaining items in the
/// stream are consumed.
pub struct VecWithoutLen<T> {
    data: Vec<T>,
}

impl<T> VecWithoutLen<T> {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn into_vec(self) -> Vec<T> {
        self.data
    }
}

impl<T> From<VecWithoutLen<T>> for Vec<T> {
    fn from(value: VecWithoutLen<T>) -> Self {
        value.data
    }
}

impl<T> From<Vec<T>> for VecWithoutLen<T> {
    fn from(value: Vec<T>) -> Self {
        Self { data: value }
    }
}

impl<T> Deref for VecWithoutLen<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for VecWithoutLen<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<T> Clone for VecWithoutLen<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self { data: self.data.clone() }
    }
}

impl<T, Item> Serialize<Item> for VecWithoutLen<T>
where
    T: Serialize<Item>,
    Error: From<<T as Serialize<Item>>::Error>,
{
    type Error = Error;
    fn serialize(&self, stream: &mut OutputStream<Item>) -> Result<(), Self::Error> {
        let mut idx = 0_usize;
        for value in &self.data {
            annotate_field(value.serialize(stream), format!("data[{}]", idx))?;
            idx += 1;
        }
        Ok(())
    }
}

impl<T, Item> Deserialize<Item> for VecWithoutLen<T>
where
    T: Deserialize<Item>,
    Error: From<<T as Deserialize<Item>>::Error>,
{
    type Error = Error;
    fn deserialize(stream: &mut InputStream<Item>) -> Result<VecWithoutLen<T>, Self::Error> {
        let mut data = Vec::<T>::new();
        while stream.stream_position() < stream.stream_len() {
            let item = annotate_field(T::deserialize(stream), format!("data[{}]", data.len()))?;
            data.push(item);
        }
        Ok(VecWithoutLen::from(data))
    }
}

impl<T> IntoIterator for VecWithoutLen<T> {
    type Item = <Vec<T> as IntoIterator>::Item;
    type IntoIter = <Vec<T> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_with_len() {
        let input = VecWithoutLen::<u8>::from(vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        let mut os = OutputStream::<u8>::new();
        input.serialize(&mut os).unwrap();
        let mut is = InputStream::from(os.take());
        let output = VecWithoutLen::<u8>::deserialize(&mut is).unwrap();
        assert_eq!(*output, *input);
    }
}
