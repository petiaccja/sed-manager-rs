use sed_manager_macros::EnumerationType;

#[repr(u8)]
#[derive(EnumerationType, PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub enum ResetType {
    PowerCycle = 0,
    Hardware = 1,
    HotPlug = 2,
    #[fallback]
    Unknown = 31,
}

pub type ResetTypes = super::super::basic_types::Set<ResetType>;
