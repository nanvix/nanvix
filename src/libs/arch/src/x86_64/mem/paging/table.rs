// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use crate::x86::mem::paging::TableIndex;

//==================================================================================================
// Virtual Address Index Extraction (x86_64 only)
//==================================================================================================

/// Extracts the PML4 index (bits 39-47) from a virtual address.
pub const fn pml4_index(vaddr: usize) -> TableIndex {
    match TableIndex::new((vaddr >> 39) & 0x1FF) {
        Some(idx) => idx,
        None => unreachable!(),
    }
}

/// Extracts the PDPT index (bits 30-38) from a virtual address.
pub const fn pdpt_index(vaddr: usize) -> TableIndex {
    match TableIndex::new((vaddr >> 30) & 0x1FF) {
        Some(idx) => idx,
        None => unreachable!(),
    }
}
