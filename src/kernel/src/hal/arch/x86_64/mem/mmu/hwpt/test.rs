// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    read_entry,
    ADDR_MASK_4K,
    ENTRIES_PER_TABLE,
    PDE_PS,
    PTE_PRESENT,
    PTE_USER,
    PTE_WRITABLE,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Reads a 4 KiB user mapping's physical frame and effective writable permission for kernel tests.
///
/// Returns `None` for missing, supervisor-only, or large-page mappings. Every paging level must
/// allow user access, and writable permission is the conjunction of all four levels.
///
/// # Safety
///
/// `pml4` and its reachable tables must be valid, identity-mapped page tables. The caller must
/// prevent concurrent table modification or destruction.
pub(crate) unsafe fn query_user_page(pml4: u64, vaddr: usize) -> Option<(usize, bool)> {
    let mut table: u64 = pml4;
    let mut writable: bool = true;
    for shift in [39, 30, 21, 12] {
        if table == 0 {
            return None;
        }
        let entry: u64 = read_entry(table, (vaddr >> shift) & (ENTRIES_PER_TABLE - 1));
        if entry & (PTE_PRESENT | PTE_USER) != (PTE_PRESENT | PTE_USER) {
            return None;
        }
        if matches!(shift, 30 | 21) && entry & PDE_PS != 0 {
            return None;
        }
        writable &= entry & PTE_WRITABLE != 0;
        table = entry & ADDR_MASK_4K;
    }
    Some((table as usize, writable))
}
