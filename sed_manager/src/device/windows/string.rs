pub fn null_terminated_to_string(ptr: *const u16) -> Result<String, std::string::FromUtf16Error> {
    let mut chars = Vec::<u16>::new();
    let mut idx = 0_usize;
    unsafe {
        while ptr.add(idx).read() != 0 {
            chars.push(ptr.add(idx).read());
            idx += 1;
        }
    }
    String::from_utf16(&chars)
}

pub fn string_to_wchars(s: &str) -> Vec<u16> {
    let mut chars: Vec<_> = s.encode_utf16().collect();
    chars.push(0);
    chars
}
