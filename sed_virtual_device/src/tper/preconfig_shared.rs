use sed_packet::{MaxBytes, Object, ObjectUid};

use crate::tper::security_provider::Table;

// This is very annoying, unsafe should be absolutely unnecessary here, and
// this should be rewritten if proper const programming are available in Rust.
pub const INITIAL_SID_PASSWORD: MaxBytes<32> =
    unsafe { MaxBytes::from_const_with_len_unchecked(*b"password                        ", 8) };
pub const PSID_PASSWORD: MaxBytes<32> =
    unsafe { MaxBytes::from_const_with_len_unchecked(*b"recovery                        ", 8) };

pub trait AllColumns {
    fn all_columns() -> impl Iterator<Item = u16>;
}

impl<O: Object> AllColumns for O {
    fn all_columns() -> impl Iterator<Item = u16> {
        0..O::FIELD_COUNT
    }
}

pub trait IntoTable<O: Object> {
    fn into_table(self) -> Option<Table<O>>;
}

impl<C> IntoTable<<C as IntoIterator>::Item> for C
where
    C: IntoIterator,
    <C as IntoIterator>::Item: Object + ObjectUid,
    <<C as IntoIterator>::Item as Object>::Ref: Ord,
{
    fn into_table(self) -> Option<Table<<C as IntoIterator>::Item>> {
        self.into_iter().map(|object| object.uid().map(|uid| (uid, object))).collect()
    }
}
