use super::{annotate_field, Deserialize, Error, InputStream, OutputStream, Serialize};
use std::{io::Seek, marker::PhantomData, ops::Deref, ops::DerefMut};

/// A vector of `T` with special a serialization format.
///
/// The elements are serialized consecutively, but instead of writing the number
/// of items as usual serializers, the number of bytes is written in a specific
/// integer format `L`.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct VecWithLen<T, L: TryFrom<usize> + TryInto<usize>> {
    data: Vec<T>,
    phantom_data: std::marker::PhantomData<L>,
}

impl<T, L: TryFrom<usize> + TryInto<usize>> VecWithLen<T, L> {
    pub const fn new() -> Self {
        Self { data: Vec::new(), phantom_data: PhantomData }
    }

    pub fn into_vec(self) -> Vec<T> {
        self.data
    }
}

impl<T, L: TryFrom<usize> + TryInto<usize>> From<VecWithLen<T, L>> for Vec<T> {
    fn from(value: VecWithLen<T, L>) -> Self {
        value.into_vec()
    }
}

impl<T, L: TryFrom<usize> + TryInto<usize>> From<Vec<T>> for VecWithLen<T, L> {
    fn from(value: Vec<T>) -> Self {
        Self { data: value, phantom_data: PhantomData }
    }
}

impl<T, L> Deref for VecWithLen<T, L>
where
    L: TryFrom<usize> + TryInto<usize>,
{
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T, L> DerefMut for VecWithLen<T, L>
where
    L: TryFrom<usize> + TryInto<usize>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<T, L: TryFrom<usize> + TryInto<usize>> Serialize<u8> for VecWithLen<T, L>
where
    T: Serialize<u8>,
    L: Serialize<u8>,
    Error: From<<T as Serialize<u8>>::Error>,
    Error: From<<L as Serialize<u8>>::Error>,
{
    type Error = Error;
    fn serialize(&self, stream: &mut OutputStream<u8>) -> Result<(), Self::Error> {
        let len_pos = stream.stream_position()?;

        let Ok(zero) = L::try_from(0usize) else {
            return annotate_field(Err(Error::InvalidData), "length_placeholder".into());
        };

        annotate_field(zero.serialize(stream), "length_placeholder".into())?;

        let data_pos = stream.stream_position()?;
        let mut idx = 0_usize;
        for value in &self.data {
            annotate_field(value.serialize(stream), format!("data[{}]", idx))?;
            idx += 1;
        }

        let end_pos = stream.stream_position()?;
        let value_len = end_pos - data_pos;
        stream.seek(std::io::SeekFrom::Start(len_pos))?;
        let Ok(value_len) = L::try_from(value_len as usize) else {
            return annotate_field(Err(Error::InvalidData), "length".into());
        };
        annotate_field(value_len.serialize(stream), "length".into())?;
        stream.seek(std::io::SeekFrom::Start(end_pos))?;
        Ok(())
    }
}

impl<T, L: TryFrom<usize> + TryInto<usize>> Deserialize<u8> for VecWithLen<T, L>
where
    T: Deserialize<u8>,
    L: Deserialize<u8>,
    Error: From<<T as Deserialize<u8>>::Error>,
    Error: From<<L as Deserialize<u8>>::Error>,
{
    type Error = Error;
    fn deserialize(stream: &mut InputStream<u8>) -> Result<VecWithLen<T, L>, Self::Error> {
        let len = annotate_field(L::deserialize(stream), "length".into())?;
        let Ok(len) = TryInto::<usize>::try_into(len) else {
            return annotate_field(Err(Error::InvalidData), "length".into());
        };
        let data_pos = stream.stream_position().unwrap();
        let end_pos = data_pos + len as u64;
        let mut data = Vec::<T>::new();
        while stream.stream_position().unwrap() < end_pos {
            let item = annotate_field(T::deserialize(stream), format!("data[{}]", data.len()))?;
            data.push(item);
        }
        if stream.stream_position().unwrap() != end_pos {
            return annotate_field(Err(Error::InvalidData), "data".into());
        }
        Ok(VecWithLen::from(data))
    }
}

impl<T, L: TryFrom<usize> + TryInto<usize>> IntoIterator for VecWithLen<T, L> {
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
        let input = VecWithLen::<u8, u32>::from(vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        let mut os = OutputStream::<u8>::new();
        input.serialize(&mut os).unwrap();
        0xCCCCCCCCu32.serialize(&mut os).unwrap();
        let mut is = InputStream::from(os.take());
        let output = VecWithLen::<u8, u32>::deserialize(&mut is).unwrap();
        assert_eq!(is.stream_position().unwrap(), 9);
        assert_eq!(*output, *input);
    }
}
