use crate::messaging::uid::TableUID;
use crate::spec::column_types::SPRef;

use super::objects::{AuthorityTable, CPinTable};
use super::table::BasicTable;

pub trait SecurityProvider {
    fn uid(&self) -> SPRef;
    fn get_table(&self, uid: TableUID) -> Option<&dyn BasicTable>;
    fn get_table_mut(&mut self, uid: TableUID) -> Option<&mut dyn BasicTable>;
    fn get_c_pin_table(&self) -> Option<&CPinTable>;
    fn get_authority_table(&self) -> Option<&AuthorityTable>;
}
