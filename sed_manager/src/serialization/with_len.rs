use super::{Deserialize, InputStream, OutputStream, Serialize, SerializeError};
use std::{io::Seek, marker::PhantomData};

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
    pub fn as_ref(&self) -> &Vec<T> {
        &self.data
    }
    pub fn as_mut_ref(&mut self) -> &mut Vec<T> {
        &mut self.data
    }
}

impl<T, L: TryFrom<usize> + TryInto<usize>> Serialize<WithLen<T, L>, u8> for WithLen<T, L>
where
    T: Serialize<T, u8>,
    L: Serialize<L, u8>,
    SerializeError: From<<T as Serialize<T, u8>>::Error>,
    SerializeError: From<<L as Serialize<L, u8>>::Error>,
{
    type Error = SerializeError;
    fn serialize(&self, stream: &mut OutputStream<u8>) -> Result<(), Self::Error> {
        let len_pos = stream.stream_position().unwrap();
        let Ok(zero) = L::try_from(0usize) else {
            return Err(SerializeError::InvalidRepresentation);
        };
        zero.serialize(stream)?;
        let data_pos = stream.stream_position().unwrap();
        for value in &self.data {
            value.serialize(stream)?;
        }
        let end_pos = stream.stream_position().unwrap();
        let value_len = end_pos - data_pos;
        stream.seek(std::io::SeekFrom::Start(len_pos)).unwrap();
        let Ok(value_len) = L::try_from(value_len as usize) else {
            return Err(SerializeError::InvalidRepresentation);
        };
        value_len.serialize(stream)?;
        stream.seek(std::io::SeekFrom::Start(end_pos)).unwrap();
        Ok(())
    }
}

impl<T, L: TryFrom<usize> + TryInto<usize>> Deserialize<WithLen<T, L>, u8> for WithLen<T, L>
where
    T: Deserialize<T, u8>,
    L: Deserialize<L, u8>,
    SerializeError: From<<T as Deserialize<T, u8>>::Error>,
    SerializeError: From<<L as Deserialize<L, u8>>::Error>,
{
    type Error = SerializeError;
    fn deserialize(stream: &mut InputStream<u8>) -> Result<WithLen<T, L>, Self::Error> {
        let len = L::deserialize(stream)?;
        let Ok(len) = TryInto::<usize>::try_into(len) else {
            return Err(SerializeError::InvalidRepresentation);
        };
        let data_pos = stream.stream_position().unwrap();
        let end_pos = data_pos + len as u64;
        let mut data = Vec::<T>::new();
        while stream.stream_position().unwrap() < end_pos {
            match T::deserialize(stream) {
                Ok(value) => data.push(value),
                Err(err) => return Err(err.into()),
            };
        }
        if stream.stream_position().unwrap() != end_pos {
            return Err(SerializeError::InvalidRepresentation); // We've overshot the actual length.
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
        assert_eq!(output.as_ref(), input.as_ref());
    }
}
