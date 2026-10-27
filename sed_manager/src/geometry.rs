//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::cmp::max;

/// The geometry of a device, or in the case of NVMe, a namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Geometry {
    /// The size of a logical sector in bytes.
    /// Not to be confused with physical sectors, e.g. for 512e drives.
    pub logical_sector_size: u32,
    /// The total number of logical sectors.
    pub logical_sector_count: u64,
}

/// The alignment requirements of locking ranges with respect to logical sectors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alignment {
    /// True if the locking ranges need to be aligned.
    pub alignment_required: bool,
    /// The locking ranges have to be aligned to groups of this many sectors.
    pub alignment_granularity: u64,
    /// The first alignment group begins at this LBA.
    pub lowest_aligned_lba: u64,
}

impl Alignment {
    /// Rounds the LBA down to the closest LBA that is aligned properly.
    pub fn align_lba(&self, lba: u64) -> u64 {
        // The LBA 0 does not have to be aligned as per the Opal V2 specification.
        if !self.alignment_required || lba == 0 {
            return lba;
        }

        let clamped = max(lba, self.lowest_aligned_lba);
        let rebased = clamped - self.lowest_aligned_lba;
        let snapped = rebased / self.alignment_granularity * self.alignment_granularity;
        snapped + self.lowest_aligned_lba
    }

    /// Check if the LBA respects the alignment requirements.
    pub fn is_lba_aligned(&self, lba: u64) -> bool {
        self.align_lba(lba) == lba
    }
}

impl Default for Alignment {
    fn default() -> Self {
        Self { alignment_required: false, alignment_granularity: 1, lowest_aligned_lba: 0 }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    // Only because writing it constructor out in rstest cases is too long.
    fn alignment(alignment_required: bool, alignment_granularity: u64, lowest_aligned_lba: u64) -> Alignment {
        Alignment { alignment_required, alignment_granularity, lowest_aligned_lba }
    }

    #[rstest]
    #[case::not_required_is_a_no_op(12345, alignment(false, 8, 0), 12345)]
    #[case::already_aligned_is_unchanged(16, alignment(true, 8, 0), 16)]
    #[case::rounds_down_when_below_midpoint(18, alignment(true, 8, 0), 16)]
    // Must always round down, even past the midpoint: alignment is applied after capping to the
    // total sector count, and rounding up could push the value back over that cap.
    #[case::rounds_down_when_above_midpoint(21, alignment(true, 8, 0), 16)]
    // Aligned LBAs are 1, 2001, 4001, ... (granularity 2000, lowest aligned LBA 1).
    #[case::respects_lowest_aligned_lba(3500, alignment(true, 2000, 1), 2001)]
    #[case::starts_at_lowest_aligned_lba(1, alignment(true, 8, 6), 6)]
    #[case::zero_always_accepted(0, alignment(true, 8, 6), 0)]
    fn align_lba(#[case] lba: u64, #[case] alignment: Alignment, #[case] expected: u64) {
        assert_eq!(alignment.align_lba(lba), expected);
        assert!(alignment.is_lba_aligned(alignment.align_lba(lba)));
    }

    #[rstest]
    #[case::not_required(19, alignment(false, 8, 4), true)]
    #[case::zero(0, alignment(true, 8, 4), true)]
    #[case::below_lowest_aligned_lba(2, alignment(true, 8, 4), false)]
    #[case::at_lowest_aligned_lba(4, alignment(true, 8, 4), true)]
    #[case::aligned(20, alignment(true, 8, 4), true)]
    #[case::unaligned(16, alignment(true, 8, 4), false)]
    fn is_lba_aligned(#[case] lba: u64, #[case] alignment: Alignment, #[case] expected: bool) {
        assert_eq!(alignment.is_lba_aligned(lba), expected);
    }
}
