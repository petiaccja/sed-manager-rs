use super::{annotate_field, Deserialize, Error, InputStream, OutputStream, Serialize};
use std::{io::Seek, marker::PhantomData, ops::Deref, ops::DerefMut};

/// A vector of `T` with special a serialization format.
///
/// The elements are serialized consecutively, but instead of writing the number
/// of items as usual serializers, the number of bytes is written in a specific
/// integer format `L`.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ItemWithLen<T, L: TryFrom<usize> + TryInto<usize>> {
    data: T,
    phantom_data: std::marker::PhantomData<L>,
}

impl<T, L: TryFrom<usize> + TryInto<usize>> ItemWithLen<T, L> {
    pub fn into_item(self) -> T {
        self.data
    }
}

impl<T, L: TryFrom<usize> + TryInto<usize>> From<T> for ItemWithLen<T, L> {
    fn from(value: T) -> Self {
        Self { data: value, phantom_data: PhantomData }
    }
}

impl<T, L> Deref for ItemWithLen<T, L>
where
    L: TryFrom<usize> + TryInto<usize>,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T, L> DerefMut for ItemWithLen<T, L>
where
    L: TryFrom<usize> + TryInto<usize>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<T, L: TryFrom<usize> + TryInto<usize>> Serialize<u8> for ItemWithLen<T, L>
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
        annotate_field(self.data.serialize(stream), "item".into())?;

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

impl<T, L: TryFrom<usize> + TryInto<usize>> Deserialize<u8> for ItemWithLen<T, L>
where
    T: Deserialize<u8>,
    L: Deserialize<u8>,
    Error: From<<T as Deserialize<u8>>::Error>,
    Error: From<<L as Deserialize<u8>>::Error>,
{
    type Error = Error;
    fn deserialize(stream: &mut InputStream<u8>) -> Result<Self, Self::Error> {
        let len = annotate_field(L::deserialize(stream), "length".into())?;
        let Ok(len) = TryInto::<usize>::try_into(len) else {
            return annotate_field(Err(Error::InvalidData), "length".into());
        };
        let data_pos = stream.stream_position().unwrap();
        let end_pos = data_pos + len as u64;
        let item = annotate_field(T::deserialize(stream), "item".into())?;
        if stream.stream_position().unwrap() != end_pos {
            return annotate_field(Err(Error::InvalidData), "data".into());
        }
        Ok(ItemWithLen::from(item))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_with_len() {
        let input = ItemWithLen::<u8, u32>::from(0xFF);
        let mut os = OutputStream::<u8>::new();
        input.serialize(&mut os).unwrap();
        0xCCCCCCCCu32.serialize(&mut os).unwrap();
        let mut is = InputStream::from(os.take());
        let output = ItemWithLen::<u8, u32>::deserialize(&mut is).unwrap();
        assert_eq!(is.stream_position().unwrap(), 5);
        assert_eq!(*output, *input);
    }
}
