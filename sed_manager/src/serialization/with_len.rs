use super::{annotate_field, Deserialize, Error, InputStream, OutputStream, Serialize};
use std::{io::Seek, marker::PhantomData, ops::Deref, ops::DerefMut};

/// A vector of `T` with special a serialization format.
///
/// The elements are serialized consecutively, but instead of writing the number
/// of items as usual serializers, the number of bytes is written in a specific
/// integer format `L`.
pub struct WithLen<T, L: TryFrom<usize> + TryInto<usize>> {
    data: Vec<T>,
    phantom_data: std::marker::PhantomData<L>,
}

impl<T, L: TryFrom<usize> + TryInto<usize>> WithLen<T, L> {
    pub fn new(value: Vec<T>) -> Self {
        Self { data: value, phantom_data: PhantomData }
    }

    pub fn into_vec(self) -> Vec<T> {
        self.data
    }
}

impl<T, L: TryFrom<usize> + TryInto<usize>> From<WithLen<T, L>> for Vec<T> {
    fn from(value: WithLen<T, L>) -> Self {
        value.data
    }
}

impl<T, L: TryFrom<usize> + TryInto<usize>> From<Vec<T>> for WithLen<T, L> {
    fn from(value: Vec<T>) -> Self {
        Self { data: value, phantom_data: PhantomData }
    }
}

impl<T, L> Deref for WithLen<T, L>
where
    L: TryFrom<usize> + TryInto<usize>,
{
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T, L> DerefMut for WithLen<T, L>
where
    L: TryFrom<usize> + TryInto<usize>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<T, L> Clone for WithLen<T, L>
where
    L: TryFrom<usize> + TryInto<usize>,
    T: Clone,
    L: Clone,
{
    fn clone(&self) -> Self {
        Self { data: self.data.clone(), phantom_data: self.phantom_data }
    }
}

impl<T, L: TryFrom<usize> + TryInto<usize>> Serialize<WithLen<T, L>, u8> for WithLen<T, L>
where
    T: Serialize<T, u8>,
    L: Serialize<L, u8>,
    Error: From<<T as Serialize<T, u8>>::Error>,
    Error: From<<L as Serialize<L, u8>>::Error>,
{
    type Error = Error;
    fn serialize(&self, stream: &mut OutputStream<u8>) -> Result<(), Self::Error> {
        let len_pos = stream.stream_position()?;

        let Ok(zero) = L::try_from(0usize) else {
            return annotate_field(
                Err(Error::io(std::io::ErrorKind::InvalidData.into(), Some(len_pos))),
                "length_placeholder".into(),
            );
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
            return annotate_field(
                Err(Error::io(std::io::ErrorKind::InvalidData.into(), Some(len_pos))),
                "length".into(),
            );
        };
        annotate_field(value_len.serialize(stream), "length".into())?;
        stream.seek(std::io::SeekFrom::Start(end_pos))?;
        Ok(())
    }
}

impl<T, L: TryFrom<usize> + TryInto<usize>> Deserialize<WithLen<T, L>, u8> for WithLen<T, L>
where
    T: Deserialize<T, u8>,
    L: Deserialize<L, u8>,
    Error: From<<T as Deserialize<T, u8>>::Error>,
    Error: From<<L as Deserialize<L, u8>>::Error>,
{
    type Error = Error;
    fn deserialize(stream: &mut InputStream<u8>) -> Result<WithLen<T, L>, Self::Error> {
        let len = annotate_field(L::deserialize(stream), "length".into())?;
        let Ok(len) = TryInto::<usize>::try_into(len) else {
            return annotate_field(Err(Error::io(std::io::ErrorKind::InvalidData.into(), None)), "length".into());
        };
        let data_pos = stream.stream_position().unwrap();
        let end_pos = data_pos + len as u64;
        let mut data = Vec::<T>::new();
        while stream.stream_position().unwrap() < end_pos {
            let item = annotate_field(T::deserialize(stream), format!("data[{}]", data.len()))?;
            data.push(item);
        }
        if stream.stream_position().unwrap() != end_pos {
            return annotate_field(Err(Error::io(std::io::ErrorKind::InvalidData.into(), None)), "data".into());
        }
        Ok(WithLen::new(data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_with_len() {
        let input = WithLen::<u8, u32>::new(vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        let mut os = OutputStream::<u8>::new();
        input.serialize(&mut os).unwrap();
        0xCCCCCCCCu32.serialize(&mut os).unwrap();
        let mut is = InputStream::from(os.take());
        let output = WithLen::<u8, u32>::deserialize(&mut is).unwrap();
        assert_eq!(is.stream_position().unwrap(), 9);
        assert_eq!(*output, *input);
    }
}
