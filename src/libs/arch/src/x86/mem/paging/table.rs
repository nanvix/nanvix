// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::PteWord;

//==================================================================================================
// Table Entry Trait
//==================================================================================================

///
/// # Description
///
/// Trait bound for entry types that can be stored in a page table.
///
/// The raw representation uses [`PteWord`] — `u32` on x86.
///
pub trait TableEntry: Copy {
    /// Creates from a raw [`PteWord`], returning `None` if the value is invalid.
    fn from_raw(raw: PteWord) -> Option<Self>;
    /// Returns the raw [`PteWord`] representation.
    fn raw(self) -> PteWord;
}

//==================================================================================================
// Virtual Address Index Extraction
//==================================================================================================

/// Extracts the PD index (bits 22-31) from a virtual address.
pub const fn pd_index(vaddr: usize) -> usize {
    (vaddr >> crate::mem::PGTAB_SHIFT) & (crate::mem::PGTAB_SIZE / crate::mem::PAGE_SIZE - 1)
}

/// Extracts the PT index (bits 12-21) from a virtual address.
pub const fn pt_index(vaddr: usize) -> usize {
    (vaddr >> crate::mem::PAGE_SHIFT) & (crate::mem::PGTAB_SIZE / crate::mem::PAGE_SIZE - 1)
}
