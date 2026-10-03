//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Geometry {
    /// The size of a logical sector in bytes.
    /// Not to be confused with physical sectors, e.g. for 512e drives.
    pub logical_sector_size: u32,
    /// The total number of logical sectors.
    pub logical_sector_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alignment {
    /// True if the locking ranges need to be aligned.
    pub alignment_required: bool,
    /// The locking ranges have to be aligned to groups of this many sectors.
    pub alignment_granularity: u64,
    /// The first alignment group begins at this LBA.
    pub lowest_aligned_lba: u64,
}

impl Default for Alignment {
    fn default() -> Self {
        Self { alignment_required: false, alignment_granularity: 1, lowest_aligned_lba: 0 }
    }
}
