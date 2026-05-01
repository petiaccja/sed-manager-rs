use core::fmt::{Debug, Display};

use sed_spec::objects::SecurityProviderRef;

pub trait Expect {
    type Inner;

    fn expect_object(self, table: impl Display, object: impl Display) -> Self::Inner;
    fn expect_serialize(self) -> Self::Inner;
    fn expect_tokenize(self) -> Self::Inner;
    fn expect_sp(self, sp: SecurityProviderRef) -> Self::Inner;
}

impl<T> Expect for Option<T> {
    type Inner = T;

    fn expect_object(self, table: impl Display, object: impl Display) -> Self::Inner {
        self.expect(&format!("internal error: expected object {table}::{object} is not present in TPer configuration"))
    }

    fn expect_serialize(self) -> Self::Inner {
        self.expect(&format!("internal error: object serialization must always succeed"))
    }

    fn expect_tokenize(self) -> Self::Inner {
        self.expect(&format!("internal error: object tokenization must always succeed"))
    }

    fn expect_sp(self, sp: SecurityProviderRef) -> Self::Inner {
        self.expect(&format!("internal error: expected TPer to have an SP with UID={sp}"))
    }
}

impl<T, E: Debug> Expect for Result<T, E> {
    type Inner = T;

    fn expect_object(self, table: impl Display, object: impl Display) -> Self::Inner {
        self.expect(&format!("internal error: expected object {table}::{object} is not present in TPer configuration"))
    }

    fn expect_serialize(self) -> Self::Inner {
        self.expect(&format!("internal error: object serialization must always succeed"))
    }

    fn expect_tokenize(self) -> Self::Inner {
        self.expect(&format!("internal error: object tokenization must always succeed"))
    }

    fn expect_sp(self, sp: SecurityProviderRef) -> Self::Inner {
        self.expect(&format!("internal error: expected TPer to have an SP with UID={sp}"))
    }
}
