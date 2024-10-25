use std::{
    io::{Seek, SeekFrom},
    ops::Range,
};

use super::stream::{InputStream, ItemWrite, OutputStream};
use bitvec::{order::Msb0, slice::BitSlice, vec::BitVec};

#[derive(Debug)]
pub enum SerializationError {
    StreamError,
    EndOfStream,
    Overflow,
}

pub trait Serialize<T, Item> {
    type Error;
    fn serialize(&self, stream: &mut OutputStream<Item>) -> Result<(), Self::Error>;
}

pub trait Deserialize<T, Item> {
    type Error;
    fn deserialize(stream: &mut InputStream<Item>) -> Result<T, Self::Error>;
}

fn reduce_field_offset(
    field_default_offset: usize,
    field_offset: &Option<usize>,
    field_bits: &Option<std::ops::Range<usize>>,
) -> (Option<usize>, Option<std::ops::Range<usize>>) {
    match &field_bits {
        Some(bits_) => {
            if bits_.start < 8 {
                (field_offset.clone(), field_bits.clone())
            } else {
                let new_start = bits_.start % 8;
                let new_end = new_start + (bits_.end - bits_.start);
                let extra_offset = bits_.start / 8;
                let new_offset = match field_offset {
                    Some(offset_) => offset_ + extra_offset,
                    None => field_default_offset + extra_offset,
                };
                (Some(new_offset), Some(Range { start: new_start, end: new_end }))
            }
        }
        None => (field_offset.clone(), field_bits.clone()),
    }
}

fn move_bits_to_range(source_bytes: &[u8], pos: &Range<usize>) -> Vec<u8> {
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
            Some(existing_byte) => *existing_byte | byte,
            None => *byte,
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
) -> Result<(), <T as Serialize<T, u8>>::Error> {
    // Stream's cursor must be left at the end of the stream after this function!
    let default_field_pos = stream.stream_position().unwrap();
    let default_offset = default_field_pos - struct_pos;
    let (offset_adj, bits_adj) = reduce_field_offset(default_offset as usize, &offset, &bits);
    let field_pos = match offset_adj {
        Some(offset_adj) => struct_pos + offset_adj as u64,
        None => default_field_pos,
    };

    let result = if let Some(bits_adj) = &bits_adj {
        let mut temp_stream = OutputStream::<u8>::new();
        match field.serialize(&mut temp_stream) {
            Ok(_) => {
                let bytes = temp_stream.as_slice();
                let moved_bytes = move_bits_to_range(bytes, &bits_adj);
                extend_with_zeros_until(stream, field_pos);
                stream.seek(SeekFrom::Start(field_pos)).unwrap();
                write_exact_bit_or(stream, moved_bytes.as_slice());
                stream.seek(SeekFrom::End(0)).unwrap();
                Ok(())
            }
            Err(err) => Err(err),
        }
    } else {
        extend_with_zeros_until(stream, field_pos);
        stream.seek(SeekFrom::Start(field_pos)).unwrap();
        let result = field.serialize(stream);
        stream.seek(SeekFrom::End(0)).unwrap();
        result
    };

    if let Some(round) = round {
        let final_pos = stream.stream_position().unwrap();
        let len = final_pos - field_pos;
        let rounded_len = (len + round as u64 - 1) / round as u64 * round as u64;
        let rounded_pos = field_pos + rounded_len;
        extend_with_zeros_until(stream, rounded_pos);
    }

    result
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
    fn reduce_field_offset_nn() {
        let (offset, bits) = reduce_field_offset(3, &None, &None);
        assert!(offset.is_none());
        assert!(bits.is_none());
    }

    #[test]
    fn reduce_field_offset_on() {
        let (offset, bits) = reduce_field_offset(3, &Some(3), &None);
        assert_eq!(offset.unwrap(), 3);
        assert!(bits.is_none());
    }

    #[test]
    fn reduce_field_offset_nb_regular() {
        let (offset, bits) = reduce_field_offset(3, &None, &Some(3..4));
        assert!(offset.is_none());
        assert_eq!(bits.unwrap(), (3..4));
    }

    #[test]
    fn reduce_field_offset_nb_overflow() {
        let (offset, bits) = reduce_field_offset(3, &None, &Some(12..15));
        assert_eq!(offset.unwrap(), 4);
        assert_eq!(bits.unwrap(), (4..7));
    }

    #[test]
    fn reduce_field_offset_ob_overflow() {
        let (offset, bits) = reduce_field_offset(3, &Some(6), &Some(12..15));
        assert_eq!(offset.unwrap(), 7);
        assert_eq!(bits.unwrap(), (4..7));
    }

    #[test]
    fn move_bits_to_range_left() {
        let bytes = [0b0000_0110, 0b0000_0001];
        let relocated = move_bits_to_range(&bytes, &(2..12));
        let expected = [0b0010_0000, 0b0001_0000];
        assert_eq!(relocated.as_ref(), expected);
    }

    #[test]
    fn move_bits_to_range_right() {
        let bytes = [0b0110_0000, 0b0000_0001];
        let relocated = move_bits_to_range(&bytes, &(7..21));
        let expected = [0b0000_0001, 0b0000_0000, 0b0000_1000];
        assert_eq!(relocated.as_ref(), expected);
    }

    #[test]
    fn serialize_at_offset_extend_zeros() {
        let field = 3_u8;
        let mut stream = OutputStream::<u8>::new();
        let struct_base = stream.stream_position().unwrap();
        let offset = Some(2);
        assert!(serialize_field(&field, &mut stream, struct_base, offset, None, None).is_ok());
        assert!(is_stream_pos_at_end(&mut stream));
        assert_eq!(stream.as_slice(), [0u8, 0u8, 3u8]);
    }

    #[test]
    fn serialize_at_offset_overwrite() {
        let field = 3_u8;
        let mut stream = OutputStream::<u8>::new();
        stream.write_exact(&[1u8, 1u8, 1u8, 1u8, 1u8]);
        stream.seek(SeekFrom::Start(0)).unwrap();
        let struct_base = stream.stream_position().unwrap();
        let offset = Some(2);
        assert!(serialize_field(&field, &mut stream, struct_base, offset, None, None).is_ok());
        assert!(is_stream_pos_at_end(&mut stream));
        assert_eq!(stream.as_slice(), [1u8, 1u8, 3u8, 1u8, 1u8]);
    }

    #[test]
    fn serialize_at_offset_bit_or() {
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
    fn serialize_at_offset_round_short() {
        let field = 0xFF_u8;
        let mut stream = OutputStream::<u8>::new();

        assert!(serialize_field(&field, &mut stream, 0, None, None, Some(4)).is_ok());
        assert!(is_stream_pos_at_end(&mut stream));
        assert_eq!(stream.as_slice(), [0xFF_u8, 0x00_u8, 0x00_u8, 0x00_u8]);
    }

    #[test]
    fn serialize_at_offset_round_long() {
        let field = 0xFFFF_FFFF_FFFF_FFFF_u64;
        let mut stream = OutputStream::<u8>::new();

        assert!(serialize_field(&field, &mut stream, 0, None, None, Some(3)).is_ok());
        assert!(is_stream_pos_at_end(&mut stream));
        assert_eq!(
            stream.as_slice(),
            [0xFF_u8, 0xFF_u8, 0xFF_u8, 0xFF_u8, 0xFF_u8, 0xFF_u8, 0xFF_u8, 0xFF_u8, 0x00_u8]
        );
    }
}
