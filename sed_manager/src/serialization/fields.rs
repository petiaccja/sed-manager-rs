use std::{
    io::{Seek, SeekFrom},
    ops::Range,
};

use super::error::Error;
use super::serialize::{Deserialize, Serialize};
use super::stream::{InputStream, ItemRead, ItemWrite, OutputStream};
use bitvec::{order::Msb0, slice::BitSlice, vec::BitVec};

fn move_end_to_range(source_bytes: &[u8], pos: &Range<usize>) -> Vec<u8> {
    let moved_len = (pos.end + 7) / 8 * 8;
    let bitfield_len = pos.len();
    let source_len = source_bytes.len() * 8;
    let source_start = source_len - bitfield_len;
    let source_bits = BitSlice::<u8, Msb0>::from_slice(source_bytes);
    let mut moved_bits = BitVec::<u8, Msb0>::new();
    moved_bits.resize(moved_len, false);
    let source_iter = source_bits[source_start..].iter().map(|x| -> bool { *x.as_ref() });
    moved_bits.splice(pos.start..pos.end, source_iter);
    moved_bits.into_vec()
}

fn move_range_to_end(source_bytes: &[u8], pos: &Range<usize>) -> Vec<u8> {
    let bitfield_len = pos.len();
    let moved_len = (bitfield_len + 7) / 8 * 8;
    let source_bits = BitSlice::<u8, Msb0>::from_slice(source_bytes);
    let mut moved_bits = BitVec::<u8, Msb0>::new();
    moved_bits.resize(moved_len, false);
    let source_iter = source_bits[pos.start..pos.end].iter().map(|x| -> bool { *x.as_ref() });
    moved_bits.splice((moved_len - bitfield_len).., source_iter);
    moved_bits.into_vec()
}

pub fn extend_with_zeros_until(stream: &mut OutputStream<u8>, stream_pos: u64) {
    if stream.seek(SeekFrom::Start(stream_pos)).is_err() {
        stream.seek(SeekFrom::End(0)).unwrap();
        while stream.stream_position().unwrap() != stream_pos {
            stream.write_one(0);
        }
    }
}

fn write_exact_bit_or(stream: &mut OutputStream<u8>, bytes: &[u8]) {
    for byte in bytes {
        let new_byte = match stream.peek_one() {
            Ok(existing_byte) => *existing_byte | byte,
            Err(_) => *byte,
        };
        stream.write_one(new_byte);
    }
}

pub fn serialize_field<T: Serialize<T, u8>>(
    field: &T,
    stream: &mut OutputStream<u8>,
    struct_pos: u64,
    offset: Option<usize>,
    bits: Option<std::ops::Range<usize>>,
    round: Option<usize>,
) -> Result<(), Error>
where
    Error: From<<T as Serialize<T, u8>>::Error>,
{
    let stream_pos = stream.stream_position()?;
    let field_pos = match offset {
        Some(offset) => struct_pos + offset as u64,
        None => stream_pos,
    };

    extend_with_zeros_until(stream, field_pos);
    stream.seek(SeekFrom::Start(field_pos))?;

    let mut bitfield_stream = OutputStream::<u8>::new();
    field.serialize(if bits.is_none() { stream } else { &mut bitfield_stream })?;

    if let Some(bits) = bits {
        let bytes = bitfield_stream.as_slice();
        let moved_bytes = move_end_to_range(bytes, &bits);
        write_exact_bit_or(stream, moved_bytes.as_slice());
    }

    if let Some(round) = round {
        let final_pos = stream.stream_position()?;
        let field_len = final_pos - field_pos;
        let rounded_len = (field_len + round as u64 - 1) / round as u64 * round as u64;
        extend_with_zeros_until(stream, field_pos + rounded_len);
    }

    Ok(())
}

pub fn deserialize_field<T: Deserialize<T, u8>>(
    stream: &mut InputStream<u8>,
    struct_pos: u64,
    offset: Option<usize>,
    bits: Option<std::ops::Range<usize>>,
    round: Option<usize>,
) -> Result<T, Error>
where
    Error: From<<T as Deserialize<T, u8>>::Error>,
{
    let stream_pos = stream.stream_position()?;
    let field_pos = match offset {
        Some(offset) => struct_pos + offset as u64,
        None => stream_pos,
    };

    stream.seek(SeekFrom::Start(field_pos))?;

    let result = if let Some(bits) = &bits {
        let bytes = stream.read_exact((bits.end + 7) / 8)?;
        let moved_bytes = move_range_to_end(bytes, bits);
        T::deserialize(&mut InputStream::<u8>::from(moved_bytes))
    } else {
        T::deserialize(stream)
    }?;

    if let Some(round) = round {
        let final_pos = stream.stream_position()?;
        let field_len = final_pos - field_pos;
        let rounded_len = (field_len + round as u64 - 1) / round as u64 * round as u64;
        stream.seek(SeekFrom::Start(field_pos + rounded_len))?;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_stream_pos_at_end<T>(stream: &mut OutputStream<T>) -> bool {
        let stream_pos = stream.stream_position().unwrap();
        stream.seek(SeekFrom::End(0)).unwrap();
        let len = stream.stream_position().unwrap();
        stream.seek(SeekFrom::Start(stream_pos)).unwrap();
        stream_pos == len
    }

    #[test]
    fn move_bits_to_range_left() {
        let bytes = [0b0000_0110, 0b0000_0001];
        let relocated = move_end_to_range(&bytes, &(2..12));
        let expected = [0b0010_0000, 0b0001_0000];
        assert_eq!(relocated.as_ref(), expected);
    }

    #[test]
    fn move_bits_to_range_right() {
        let bytes = [0b0110_0000, 0b0000_0001];
        let relocated = move_end_to_range(&bytes, &(7..21));
        let expected = [0b0000_0001, 0b0000_0000, 0b0000_1000];
        assert_eq!(relocated.as_ref(), expected);
    }

    #[test]
    fn move_bits_to_range_large() {
        let bytes = [0b0110_0000, 0b0000_0001];
        let relocated = move_end_to_range(&bytes, &(15..29));
        let expected = [0b0000_0000, 0b0000_0001, 0b0000_0000, 0b0000_1000];
        assert_eq!(relocated.as_ref(), expected);
    }

    #[test]
    fn serialize_field_simple() {
        let field = 3_u8;
        let mut stream = OutputStream::<u8>::new();
        let struct_base = stream.stream_position().unwrap();
        assert!(serialize_field(&field, &mut stream, struct_base, None, None, None).is_ok());
        assert!(is_stream_pos_at_end(&mut stream));
        assert_eq!(stream.as_slice(), [3u8]);
    }

    #[test]
    fn serialize_field_offset_extend_zeros() {
        let field = 3_u8;
        let mut stream = OutputStream::<u8>::new();
        assert!(serialize_field(&field, &mut stream, 0, Some(2), None, None).is_ok());
        assert_eq!(stream.stream_position().unwrap(), 3);
        assert_eq!(stream.as_slice(), [0u8, 0u8, 3u8]);
    }

    #[test]
    fn serialize_field_offset_overwrite() {
        let field = 3_u8;
        let mut stream = OutputStream::<u8>::new();
        stream.write_exact(&[0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8]);
        stream.seek(SeekFrom::Start(0)).unwrap();
        assert!(serialize_field(&field, &mut stream, 0, Some(2), None, None).is_ok());
        assert_eq!(stream.stream_position().unwrap(), 3);
        assert_eq!(stream.as_slice(), [0xFFu8, 0xFFu8, 3u8, 0xFFu8, 0xFFu8]);
    }

    #[test]
    fn serialize_field_offset_bit_or() {
        let field = 0b_1111_1111_u8;
        let mut stream = OutputStream::<u8>::new();
        stream.write_exact(&[0b_0000_0000_u8, 0b_0000_0000_u8]);
        stream.seek(SeekFrom::Start(0)).unwrap();

        assert!(serialize_field(&field, &mut stream, 0, Some(1), Some(2..3), None).is_ok());
        assert!(is_stream_pos_at_end(&mut stream));
        assert!(serialize_field(&field, &mut stream, 0, Some(0), Some(14..18), None).is_ok());
        assert!(is_stream_pos_at_end(&mut stream));
        assert_eq!(stream.as_slice(), [0b_0000_0000_u8, 0b_0010_0011_u8, 0b_1100_0000_u8]);
    }

    #[test]
    fn serialize_field_round_single() {
        let field = 0xFF_u8;
        let mut stream = OutputStream::<u8>::new();

        assert!(serialize_field(&field, &mut stream, 0, None, None, Some(4)).is_ok());
        assert!(is_stream_pos_at_end(&mut stream));
        assert_eq!(stream.as_slice(), [0xFF_u8, 0x00_u8, 0x00_u8, 0x00_u8]);
    }

    #[test]
    fn serialize_field_round_multiple() {
        let field = 0xFFFF_FFFF_FFFF_FFFF_u64;
        let mut stream = OutputStream::<u8>::new();

        assert!(serialize_field(&field, &mut stream, 0, None, None, Some(3)).is_ok());
        assert!(is_stream_pos_at_end(&mut stream));
        assert_eq!(
            stream.as_slice(),
            [0xFF_u8, 0xFF_u8, 0xFF_u8, 0xFF_u8, 0xFF_u8, 0xFF_u8, 0xFF_u8, 0xFF_u8, 0x00_u8]
        );
    }

    const DESERIALIZE_DATA: [u8; 9] = [
        0x01_u8, 0x23_u8, 0x45_u8, 0x67_u8, 0x89_u8, 0xAB_u8, 0xCD_u8, 0xEF_u8, 0x00_u8,
    ];

    #[test]
    fn deserialize_field_simple() {
        let mut stream = InputStream::<u8>::new(DESERIALIZE_DATA.as_slice());
        let result = deserialize_field::<u16>(&mut stream, 0, None, None, None);
        assert_eq!(result.unwrap(), 0x0123);
        assert_eq!(stream.stream_position().unwrap(), 2);
    }

    #[test]
    fn deserialize_field_offset() {
        let mut stream = InputStream::<u8>::new(DESERIALIZE_DATA.as_slice());
        let result = deserialize_field::<u16>(&mut stream, 0, Some(2), None, None);
        assert_eq!(result.unwrap(), 0x4567);
        assert_eq!(stream.stream_position().unwrap(), 4);
    }

    #[test]
    fn deserialize_field_bits() {
        let mut stream = InputStream::<u8>::new(DESERIALIZE_DATA.as_slice());
        let result = deserialize_field::<u16>(&mut stream, 0, None, Some(4..20), None);
        assert_eq!(result.unwrap(), 0x1234);
        assert_eq!(stream.stream_position().unwrap(), 3);
    }

    #[test]
    fn deserialize_field_round_single() {
        let mut stream = InputStream::<u8>::new(DESERIALIZE_DATA.as_slice());
        let result = deserialize_field::<u16>(&mut stream, 0, None, None, Some(6));
        assert_eq!(result.unwrap(), 0x0123);
        assert_eq!(stream.stream_position().unwrap(), 6);
    }

    #[test]
    fn deserialize_field_round_multiple() {
        let mut stream = InputStream::<u8>::new(DESERIALIZE_DATA.as_slice());
        let result = deserialize_field::<u64>(&mut stream, 0, None, None, Some(9));
        assert_eq!(result.unwrap(), 0x0123456789ABCDEF);
        assert_eq!(stream.stream_position().unwrap(), 9);
    }
}
