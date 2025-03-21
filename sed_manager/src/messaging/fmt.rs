//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::fmt::Write;

#[allow(unused)]
macro_rules! format_flat {
    ($value:expr) => {{
        let mut s = String::new();
        let mut f = PrettyFormatter::new(&mut s, None);
        let _ = $value.fmt(&mut f);
        s
    }};
}

#[allow(unused)]
macro_rules! format_indented {
    ($value:expr, $indent:expr) => {{
        let mut s = String::new();
        let mut f = ::sed_manager::messaging::fmt::PrettyFormatter::new(&mut s, Some($indent));
        let _ = $value.fmt(&mut f);
        s
    }};
}

#[allow(unused)]
pub(crate) use format_flat;
#[allow(unused)]
pub(crate) use format_indented;

pub struct PrettyFormatter<'a> {
    indent: Option<usize>,
    level: usize,
    line_start: bool,
    buf: &'a mut (dyn Write + 'a),
}

pub trait PrettyPrint {
    fn fmt(&self, f: &mut PrettyFormatter<'_>) -> Result<(), core::fmt::Error>;
}

impl<'a> PrettyFormatter<'a> {
    pub fn new(buf: &'a mut (dyn Write + 'a), indent: Option<usize>) -> Self {
        Self { indent, level: 0, line_start: true, buf }
    }

    pub fn indented(
        &mut self,
        f: impl Fn(&mut PrettyFormatter) -> Result<(), core::fmt::Error>,
    ) -> Result<(), core::fmt::Error> {
        self.level += 1;
        let res = f(self);
        self.level -= 1;
        res
    }

    pub fn is_indenting_enabled(&self) -> bool {
        self.indent.is_some()
    }

    fn write_indentation(&mut self) -> Result<(), core::fmt::Error> {
        if let Some(indent) = self.indent {
            for _ in 0..(indent * self.level) {
                self.write_char(' ')?;
            }
        }
        Ok(())
    }
}

impl<'a> PrettyFormatter<'a> {
    pub fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error> {
        let mut chunks = s.split('\n');
        let mut first = chunks.next();
        while let Some(chunk) = first {
            let next = chunks.next();

            if !chunk.is_empty() {
                if core::mem::replace(&mut self.line_start, false) {
                    self.write_indentation()?;
                }
                self.buf.write_str(chunk)?;
            }

            if next.is_some() {
                self.write_char('\n')?;
                self.line_start = true;
            }

            first = next;
        }
        Ok(())
    }

    pub fn write_char(&mut self, c: char) -> Result<(), core::fmt::Error> {
        if core::mem::replace(&mut self.line_start, false) {
            self.write_indentation()?;
        }
        self.buf.write_char(c)?;
        self.line_start = c == '\n';
        Ok(())
    }
}

impl PrettyPrint for &str {
    fn fmt(&self, f: &mut PrettyFormatter<'_>) -> Result<(), core::fmt::Error> {
        f.write_str(self)
    }
}

impl PrettyPrint for char {
    fn fmt(&self, f: &mut PrettyFormatter<'_>) -> Result<(), core::fmt::Error> {
        f.write_char(*self)
    }
}

impl PrettyPrint for String {
    fn fmt(&self, f: &mut PrettyFormatter<'_>) -> Result<(), core::fmt::Error> {
        f.write_str(&self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Number(u64);
    struct Nested(Vec<Number>);

    impl PrettyPrint for Number {
        fn fmt(&self, f: &mut PrettyFormatter<'_>) -> Result<(), core::fmt::Error> {
            f.write_str(&format!("{}", self.0))
        }
    }

    impl PrettyPrint for Nested {
        fn fmt(&self, f: &mut PrettyFormatter<'_>) -> Result<(), core::fmt::Error> {
            let sep = if f.is_indenting_enabled() { '\n' } else { ' ' };
            f.write_char('[')?;
            f.write_char(sep)?;
            f.indented(|f| {
                for item in &self.0 {
                    item.fmt(f)?;
                    f.write_char(',')?;
                    f.write_char(sep)?;
                }
                Ok(())
            })?;
            f.write_char(']')?;
            Ok(())
        }
    }

    #[test]
    fn pretty_print_number() {
        assert_eq!(&format_flat!(Number(5)), "5");
    }

    #[test]
    fn pretty_print_vector_flat() {
        let value = Nested(vec![Number(5), Number(6)]);
        let expected = "[ 5, 6, ]";
        assert_eq!(&format_flat!(value), expected);
    }

    #[test]
    fn pretty_print_vector_indented() {
        let value = Nested(vec![Number(5), Number(6)]);
        let expected = "[\n  5,\n  6,\n]";
        assert_eq!(&format_indented!(value, 2), expected);
    }
}
