use super::{LongAtom, MediumAtom, ShortAtom, TinyAtom, Token};

macro_rules! impl_from_int {
    ($int:ty) => {
        impl From<$int> for Token {
            fn from(value: $int) -> Self {
                if let Ok(atom) = TinyAtom::try_from(value) {
                    Token::TinyAtom(atom)
                } else {
                    Token::ShortAtom(ShortAtom::from(value))
                }
            }
        }
    };
}

impl_from_int!(i8);
impl_from_int!(i16);
impl_from_int!(i32);
impl_from_int!(i64);
impl_from_int!(u8);
impl_from_int!(u16);
impl_from_int!(u32);
impl_from_int!(u64);

macro_rules! impl_try_into_int {
    ($int:ty) => {
        impl<'a> TryFrom<&'a Token> for $int {
            type Error = &'a Token;

            fn try_from(value: &'a Token) -> Result<Self, Self::Error> {
                match value {
                    Token::TinyAtom(atom) => Self::try_from(atom).map_err(|_| value),
                    Token::ShortAtom(atom) => Self::try_from(atom).map_err(|_| value),
                    Token::MediumAtom(atom) => Self::try_from(atom).map_err(|_| value),
                    Token::LongAtom(atom) => Self::try_from(atom).map_err(|_| value),
                    _ => Err(value),
                }
            }
        }

        impl TryFrom<Token> for $int {
            type Error = Token;

            fn try_from(value: Token) -> Result<Self, Self::Error> {
                match &value {
                    Token::TinyAtom(atom) => Self::try_from(atom).map_err(|_| ()),
                    Token::ShortAtom(atom) => Self::try_from(atom).map_err(|_| ()),
                    Token::MediumAtom(atom) => Self::try_from(atom).map_err(|_| ()),
                    Token::LongAtom(atom) => Self::try_from(atom).map_err(|_| ()),
                    _ => Err(()),
                }
                .map_err(|_| value)
            }
        }
    };
}

impl_try_into_int!(i8);
impl_try_into_int!(i16);
impl_try_into_int!(i32);
impl_try_into_int!(i64);
impl_try_into_int!(u8);
impl_try_into_int!(u16);
impl_try_into_int!(u32);
impl_try_into_int!(u64);

impl<'a> TryFrom<&'a [u8]> for Token {
    type Error = &'a [u8];

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        ShortAtom::try_from(value)
            .map(|atom| Self::ShortAtom(atom))
            .or_else(|value| MediumAtom::try_from(value).map(|atom| Self::MediumAtom(atom)))
            .or_else(|value| LongAtom::try_from(value).map(|atom| Self::LongAtom(atom)))
    }
}

impl TryFrom<Vec<u8>> for Token {
    type Error = Vec<u8>;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        ShortAtom::try_from(value)
            .map(|atom| Self::ShortAtom(atom))
            .or_else(|value| MediumAtom::try_from(value).map(|atom| Self::MediumAtom(atom)))
            .or_else(|value| LongAtom::try_from(value).map(|atom| Self::LongAtom(atom)))
    }
}

impl<'a> TryFrom<&'a Token> for &'a [u8] {
    type Error = &'a Token;

    fn try_from(value: &'a Token) -> Result<Self, Self::Error> {
        match value {
            Token::ShortAtom(atom) => Self::try_from(atom).map_err(|_| value),
            Token::MediumAtom(atom) => Self::try_from(atom).map_err(|_| value),
            Token::LongAtom(atom) => Self::try_from(atom).map_err(|_| value),
            _ => Err(value),
        }
    }
}

impl<'a> TryFrom<&'a Token> for Vec<u8> {
    type Error = &'a Token;

    fn try_from(value: &'a Token) -> Result<Self, Self::Error> {
        match value {
            Token::ShortAtom(atom) => Self::try_from(atom).map_err(|_| value),
            Token::MediumAtom(atom) => Self::try_from(atom).map_err(|_| value),
            Token::LongAtom(atom) => Self::try_from(atom).map_err(|_| value),
            _ => Err(value),
        }
    }
}

impl TryFrom<Token> for Vec<u8> {
    type Error = Token;

    fn try_from(value: Token) -> Result<Self, Self::Error> {
        match value {
            Token::ShortAtom(atom) => Self::try_from(atom).map_err(|atom| Token::ShortAtom(atom)),
            Token::MediumAtom(atom) => Self::try_from(atom).map_err(|atom| Token::MediumAtom(atom)),
            Token::LongAtom(atom) => Self::try_from(atom).map_err(|atom| Token::LongAtom(atom)),
            _ => Err(value),
        }
    }
}
