//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

pub trait ToNullTerminated {
    #[allow(unused)]
    fn to_null_terminated_utf16(&self) -> Vec<u16>;
    #[allow(unused)]
    fn to_null_terminated_utf8(&self) -> Vec<u8>;
}

pub trait FromNullTerminated
where
    Self: Sized,
{
    #[allow(unused)]
    fn from_null_terminated_utf16(value: *const u16) -> Option<Self>;
    #[allow(unused)]
    fn from_null_terminated_utf8(value: *const u8) -> Option<Self>;
}

impl ToNullTerminated for &str {
    fn to_null_terminated_utf16(&self) -> Vec<u16> {
        let mut characters: Vec<_> = self.encode_utf16().collect();
        characters.push(0);
        characters
    }
    fn to_null_terminated_utf8(&self) -> Vec<u8> {
        let mut characters: Vec<_> = self.bytes().collect();
        characters.push(0);
        characters
    }
}

impl ToNullTerminated for String {
    fn to_null_terminated_utf16(&self) -> Vec<u16> {
        self.as_str().to_null_terminated_utf16()
    }
    fn to_null_terminated_utf8(&self) -> Vec<u8> {
        self.as_str().to_null_terminated_utf8()
    }
}

impl FromNullTerminated for String {
    fn from_null_terminated_utf16(value: *const u16) -> Option<Self> {
        Self::from_utf16(pointer_to_slice(value)).ok()
    }
    fn from_null_terminated_utf8(value: *const u8) -> Option<Self> {
        Self::from_utf8(Vec::from(pointer_to_slice(value))).ok()
    }
}

fn pointer_to_slice<T>(ptr: *const T) -> &'static [T]
where
    i64: From<T>,
{
    if ptr.is_null() {
        return &[];
    };
    let mut len: usize = 0;
    loop {
        let current = unsafe { ptr.add(len) };
        let value = unsafe { current.read() };
        if i64::from(value) == 0 {
            return unsafe { core::slice::from_raw_parts(ptr, len) };
        }
        len += 1;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn utf16_roundtrip() {
        let input = "kjufgseftvjsb";
        let characters = input.to_null_terminated_utf16();
        let output = String::from_null_terminated_utf16(characters.as_ptr());
        assert_eq!(input, output.unwrap());
    }

    #[test]
    fn utf8_roundtrip() {
        let input = "kjufgseftvjsb";
        let characters = input.to_null_terminated_utf8();
        let output = String::from_null_terminated_utf8(characters.as_ptr());
        assert_eq!(input, output.unwrap());
    }
}
