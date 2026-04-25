use smallvec::SmallVec;

use sed_spec_macros::{DetokenizeStruct, FieldList, Object, TokenizeStruct};

use crate::objects::{AuthorityRef, SecurityProviderRef};
use crate::preconfig::core::shared::table_id;
use crate::types::{Date, LifeCycleState};

#[derive(Debug, Clone, Default, PartialEq, Eq, Object, TokenizeStruct, DetokenizeStruct, FieldList)]
#[object(table=table_id::SP)]
pub struct SecurityProvider {
    pub uid: Option<SecurityProviderRef>,
    pub name: Option<String>,
    pub org: Option<AuthorityRef>,
    pub effective_auth: Option<SmallVec<[u8; 32]>>,
    pub date_of_issue: Option<Date>,
    pub bytes: Option<u64>,
    pub life_cycle_state: Option<LifeCycleState>,
    pub frozen: Option<bool>,
}
