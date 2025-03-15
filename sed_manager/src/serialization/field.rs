use super::{Seek, SeekFrom};
use core::ops::{BitAnd, Not, Shl};
use std::ops::Shr;

use super::error::Error;
use super::serialize::{Deserialize, Serialize};
use super::stream::{InputStream, ItemWrite, OutputStream};

fn get_position(stream: &impl Seek, base: Option<u64>, offset: Option<u64>) -> u64 {
    if let Some(offset) = offset {
        offset + base.unwrap_or(stream.stream_position())
    } else {
        stream.stream_position()
    }
}

pub fn serialize<DataType>(
    stream: &mut OutputStream<u8>,
    value: &DataType,
    base: Option<u64>,
    offset: Option<u64>,
    round: Option<u64>,
) -> Result<(), Error>
where
    DataType: Serialize<u8>,
    Error: From<<DataType as Serialize<u8>>::Error>,
{
    let position = get_position(stream, base, offset);

    extend_with_zeros_until(stream, position);
    stream.seek(SeekFrom::Start(position))?;

    value.serialize(stream)?;

    if let Some(round) = round {
        let unrounded_end = stream.stream_position();
        let unrounded_len = unrounded_end - position;
        let rounded_len = (unrounded_len + round - 1) / round * round;
        extend_with_zeros_until(stream, position + rounded_len);
    }

    Ok(())
}

pub fn deserialize<DataType>(
    stream: &mut InputStream<u8>,
    base: Option<u64>,
    offset: Option<u64>,
    round: Option<u64>,
) -> Result<DataType, Error>
where
    DataType: Deserialize<u8>,
    Error: From<<DataType as Deserialize<u8>>::Error>,
{
    let position = get_position(stream, base, offset);

    stream.seek(SeekFrom::Start(position))?;

    let value = DataType::deserialize(stream)?;

    if let Some(round) = round {
        let unrounded_end = stream.stream_position();
        let unrounded_len = unrounded_end - position;
        let rounded_len = (unrounded_len + round - 1) / round * round;
        stream.seek(SeekFrom::Start(position + rounded_len))?;
    }

    Ok(value)
}

pub fn serialize_bit_field<DataType, BitFieldType>(
    stream: &mut OutputStream<u8>,
    value: &DataType,
    base: Option<u64>,
    offset: Option<u64>,
    round: Option<u64>,
    bit_subset: core::ops::Range<u8>,
) -> Result<(), Error>
where
    BitFieldType: Serialize<u8>,
    Error: From<<BitFieldType as Serialize<u8>>::Error>,
    BitFieldType: From<DataType>,
    DataType: Clone,
    BitFieldType: Shl<BitFieldType, Output = BitFieldType>
        + BitAnd<BitFieldType, Output = BitFieldType>
        + Not<Output = BitFieldType>
        + Default
        + From<u8>,
{
    let bit_field: BitFieldType = BitFieldType::from(value.clone()) << BitFieldType::from(bit_subset.start);
    let mask = !(!BitFieldType::default() << BitFieldType::from(bit_subset.end - bit_subset.start))
        << BitFieldType::from(bit_subset.start);
    let value = mask & bit_field;

    let position = get_position(stream, base, offset);

    extend_with_zeros_until(stream, position);
    stream.seek(SeekFrom::Start(position))?;

    stream.with_overwrite(|old, new| *old |= new, |stream| value.serialize(stream))?;

    if let Some(round) = round {
        let unrounded_end = stream.stream_position();
        let unrounded_len = unrounded_end - position;
        let rounded_len = (unrounded_len + round - 1) / round * round;
        extend_with_zeros_until(stream, position + rounded_len);
    }

    Ok(())
}

pub fn deserialize_bit_field<DataType, BitFieldType>(
    stream: &mut InputStream<u8>,
    base: Option<u64>,
    offset: Option<u64>,
    round: Option<u64>,
    bit_subset: core::ops::Range<u8>,
) -> Result<DataType, Error>
where
    BitFieldType: Deserialize<u8>,
    Error: From<<BitFieldType as Deserialize<u8>>::Error>,
    BitFieldType: Shr<BitFieldType, Output = BitFieldType>
        + Shl<BitFieldType, Output = BitFieldType>
        + BitAnd<BitFieldType, Output = BitFieldType>
        + Not<Output = BitFieldType>
        + Default
        + From<u8>,
    DataType: TryFrom<BitFieldType>,
{
    let position = get_position(stream, base, offset);

    stream.seek(SeekFrom::Start(position))?;

    let bit_field = BitFieldType::deserialize(stream)?;
    let mask = !(!BitFieldType::default() << BitFieldType::from(bit_subset.end - bit_subset.start));
    let value = (bit_field >> BitFieldType::from(bit_subset.start)) & mask;

    if let Some(round) = round {
        let unrounded_end = stream.stream_position();
        let unrounded_len = unrounded_end - position;
        let rounded_len = (unrounded_len + round - 1) / round * round;
        stream.seek(SeekFrom::Start(position + rounded_len))?;
    }

    Ok(DataType::try_from(value).map_err(|_| Error::InvalidData)?)
}

/// This garbage shit would not be needed if there was some goddamn specialization.
pub fn deserialize_bit_field_bool<BitFieldType>(
    stream: &mut InputStream<u8>,
    base: Option<u64>,
    offset: Option<u64>,
    round: Option<u64>,
    bit_subset: core::ops::Range<u8>,
) -> Result<bool, Error>
where
    BitFieldType: Deserialize<u8>,
    Error: From<<BitFieldType as Deserialize<u8>>::Error>,
    BitFieldType: Shr<BitFieldType, Output = BitFieldType>
        + Shl<BitFieldType, Output = BitFieldType>
        + BitAnd<BitFieldType, Output = BitFieldType>
        + Not<Output = BitFieldType>
        + Default
        + From<u8>,
    u8: TryFrom<BitFieldType>,
{
    let value = deserialize_bit_field::<u8, BitFieldType>(stream, base, offset, round, bit_subset)?;
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(Error::InvalidData),
    }
}

pub fn extend_with_zeros_until(stream: &mut OutputStream<u8>, stream_pos: u64) {
    if stream.seek(SeekFrom::Start(stream_pos)).is_err() {
        stream.seek(SeekFrom::End(0)).unwrap();
        while stream.stream_position() != stream_pos {
            stream.write_one(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_default() -> Result<(), Error> {
        let mut stream = OutputStream::new();
        serialize(&mut stream, &0xBEEF_u16, None, None, None)?;
        assert_eq!(stream.stream_position(), 2);
        assert_eq!(stream.take(), vec![0xBE, 0xEF]);
        Ok(())
    }

    #[test]
    fn serialize_base() -> Result<(), Error> {
        let mut stream = OutputStream::new();
        [0_u8; 2].serialize(&mut stream)?;
        serialize(&mut stream, &0xBEEF_u16, Some(0), None, None)?;
        assert_eq!(stream.stream_position(), 4);
        assert_eq!(stream.take(), vec![0x00, 0x00, 0xBE, 0xEF]);
        Ok(())
    }

    #[test]
    fn serialize_offset() -> Result<(), Error> {
        let mut stream = OutputStream::new();
        [0_u8; 2].serialize(&mut stream)?;
        serialize(&mut stream, &0xBEEF_u16, None, Some(2), None)?;
        assert_eq!(stream.stream_position(), 6);
        assert_eq!(stream.take(), vec![0x00, 0x00, 0x00, 0x00, 0xBE, 0xEF]);
        Ok(())
    }

    #[test]
    fn serialize_base_offset() -> Result<(), Error> {
        let mut stream = OutputStream::new();
        [0_u8; 2].serialize(&mut stream)?;
        serialize(&mut stream, &0xBEEF_u16, Some(2), Some(1), None)?;
        assert_eq!(stream.stream_position(), 5);
        assert_eq!(stream.take(), vec![0x00, 0x00, 0x00, 0xBE, 0xEF]);
        Ok(())
    }

    #[test]
    fn serialize_round() -> Result<(), Error> {
        let mut stream = OutputStream::new();
        serialize(&mut stream, &0xBEEF_u16, None, None, Some(4))?;
        assert_eq!(stream.stream_position(), 4);
        assert_eq!(stream.take(), vec![0xBE, 0xEF, 0x00, 0x00]);
        Ok(())
    }

    #[test]
    fn deserialize_default() -> Result<(), Error> {
        let mut stream = InputStream::from(vec![0xBE, 0xEF]);
        let value = deserialize::<u16>(&mut stream, None, None, None)?;
        assert_eq!(stream.stream_position(), 2);
        assert_eq!(value, 0xBEEF_u16);
        Ok(())
    }

    #[test]
    fn deserialize_base() -> Result<(), Error> {
        let mut stream = InputStream::from(vec![0x00, 0x00, 0xBE, 0xEF]);
        <[u8; 2]>::deserialize(&mut stream)?;
        let value = deserialize::<u16>(&mut stream, Some(0), None, None)?;
        assert_eq!(stream.stream_position(), 4);
        assert_eq!(value, 0xBEEF_u16);
        Ok(())
    }

    #[test]
    fn deserialize_offset() -> Result<(), Error> {
        let mut stream = InputStream::from(vec![0x00, 0x00, 0x00, 0x00, 0xBE, 0xEF]);
        <[u8; 2]>::deserialize(&mut stream)?;
        let value = deserialize::<u16>(&mut stream, None, Some(2), None)?;
        assert_eq!(stream.stream_position(), 6);
        assert_eq!(value, 0xBEEF_u16);
        Ok(())
    }

    #[test]
    fn deserialize_base_offset() -> Result<(), Error> {
        let mut stream = InputStream::from(vec![0x00, 0x00, 0x00, 0xBE, 0xEF]);
        let value = deserialize::<u16>(&mut stream, Some(2), Some(1), None)?;
        assert_eq!(stream.stream_position(), 5);
        assert_eq!(value, 0xBEEF_u16);
        Ok(())
    }

    #[test]
    fn deserialize_round() -> Result<(), Error> {
        let mut stream = InputStream::from(vec![0xBE, 0xEF, 0x00, 0x00]);
        let value = deserialize::<u16>(&mut stream, None, None, Some(4))?;
        assert_eq!(stream.stream_position(), 4);
        assert_eq!(value, 0xBEEF_u16);
        Ok(())
    }

    #[test]
    fn serialize_bit_field_default() -> Result<(), Error> {
        let mut stream = OutputStream::new();
        serialize_bit_field::<_, u16>(&mut stream, &0b1011_u8, None, None, None, 6..10)?;
        assert_eq!(stream.stream_position(), 2);
        assert_eq!(stream.take(), vec![0b0000_0010, 0b1100_0000]);
        Ok(())
    }

    #[test]
    fn serialize_bit_field_masking() -> Result<(), Error> {
        let mut stream = OutputStream::new();
        serialize_bit_field::<_, u16>(&mut stream, &0xFFFFu16, None, None, None, 6..10)?;
        assert_eq!(stream.stream_position(), 2);
        assert_eq!(stream.take(), vec![0b0000_0011, 0b1100_0000]);
        Ok(())
    }

    #[test]
    fn serialize_bit_field_base() -> Result<(), Error> {
        let mut stream = OutputStream::new();
        [0_u8; 2].serialize(&mut stream)?;
        serialize_bit_field::<_, u16>(&mut stream, &0b1011_u8, Some(2), None, None, 6..10)?;
        assert_eq!(stream.stream_position(), 4);
        assert_eq!(stream.take(), vec![0x00, 0x00, 0b0000_0010, 0b1100_0000]);
        Ok(())
    }

    #[test]
    fn serialize_bit_field_offset() -> Result<(), Error> {
        let mut stream = OutputStream::new();
        serialize_bit_field::<_, u16>(&mut stream, &0b1011_u8, None, Some(2), None, 6..10)?;
        assert_eq!(stream.stream_position(), 4);
        assert_eq!(stream.take(), vec![0x00, 0x00, 0b0000_0010, 0b1100_0000]);
        Ok(())
    }

    #[test]
    fn serialize_bit_field_base_offset() -> Result<(), Error> {
        let mut stream = OutputStream::new();
        [0_u8; 2].serialize(&mut stream)?;
        serialize_bit_field::<_, u16>(&mut stream, &0b1011_u8, Some(2), Some(1), None, 6..10)?;
        assert_eq!(stream.stream_position(), 5);
        assert_eq!(stream.take(), vec![0x00, 0x00, 0x00, 0b0000_0010, 0b1100_0000]);
        Ok(())
    }

    #[test]
    fn serialize_bit_field_round() -> Result<(), Error> {
        let mut stream = OutputStream::new();
        serialize_bit_field::<_, u16>(&mut stream, &0b1011_u8, None, None, Some(4), 6..10)?;
        assert_eq!(stream.stream_position(), 4);
        assert_eq!(stream.take(), vec![0b0000_0010, 0b1100_0000, 0x00, 0x00]);
        Ok(())
    }

    #[test]
    fn deserialize_bit_field_default() -> Result<(), Error> {
        let mut stream = InputStream::from(vec![0b0000_0010, 0b1100_0000]);
        let value = deserialize_bit_field::<u8, u16>(&mut stream, None, None, None, 6..10)?;
        assert_eq!(stream.stream_position(), 2);
        assert_eq!(value, 0b1011_u8);
        Ok(())
    }

    #[test]
    fn deserialize_bit_field_masking() -> Result<(), Error> {
        let mut stream = InputStream::from(vec![0xFF, 0xFF]);
        let value = deserialize_bit_field::<u8, u16>(&mut stream, None, None, None, 6..10)?;
        assert_eq!(stream.stream_position(), 2);
        assert_eq!(value, 0b1111_u8);
        Ok(())
    }

    #[test]
    fn deserialize_bit_field_base() -> Result<(), Error> {
        let mut stream = InputStream::from(vec![0x00, 0x00, 0b0000_0010, 0b1100_0000]);
        <[u8; 2]>::deserialize(&mut stream)?;
        let value = deserialize_bit_field::<u8, u16>(&mut stream, Some(0), None, None, 6..10)?;
        assert_eq!(stream.stream_position(), 4);
        assert_eq!(value, 0b1011_u8);
        Ok(())
    }

    #[test]
    fn deserialize_bit_field_offset() -> Result<(), Error> {
        let mut stream = InputStream::from(vec![0x00, 0x00, 0b0000_0010, 0b1100_0000]);
        let value = deserialize_bit_field::<u8, u16>(&mut stream, None, Some(2), None, 6..10)?;
        assert_eq!(stream.stream_position(), 4);
        assert_eq!(value, 0b1011_u8);
        Ok(())
    }

    #[test]
    fn deserialize_bit_field_base_offset() -> Result<(), Error> {
        let mut stream = InputStream::from(vec![0x00, 0x00, 0x00, 0b0000_0010, 0b1100_0000]);
        let value = deserialize_bit_field::<u8, u16>(&mut stream, Some(2), Some(1), None, 6..10)?;
        assert_eq!(stream.stream_position(), 5);
        assert_eq!(value, 0b1011_u8);
        Ok(())
    }

    #[test]
    fn deserialize_bit_field_round() -> Result<(), Error> {
        let mut stream = InputStream::from(vec![0b0000_0010, 0b1100_0000, 0x00, 0x00]);
        let value = deserialize_bit_field::<u8, u16>(&mut stream, None, None, Some(4), 6..10)?;
        assert_eq!(stream.stream_position(), 4);
        assert_eq!(value, 0b1011_u8);
        Ok(())
    }
}
