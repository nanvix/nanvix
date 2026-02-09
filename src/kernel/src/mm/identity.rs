// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::arch::mem;

//==================================================================================================
// Constants
//==================================================================================================

/// Number of page tables needed to identity-map [0, MEMORY_SIZE).
const NUM_PGTABS: usize = config::kernel::MEMORY_SIZE.div_ceil(mem::PGTAB_SIZE);

/// Number of 32-bit entries per page table (and per page directory).
const ENTRIES_PER_TABLE: usize = mem::PAGE_SIZE / core::mem::size_of::<u32>();

/// PTE/PDE flags: Present + Read/Write.
const FLAGS_RW_PRESENT: u32 = 0x3;

//==================================================================================================
// Static Storage
//==================================================================================================

/// Page-aligned 4 KB table used as either a page directory or page table.
#[derive(Clone, Copy)]
#[repr(C, align(4096))]
struct AlignedTable {
    entries: [u32; ENTRIES_PER_TABLE],
}

/// Identity page directory (1024 entries, 4 KB).
static mut PGDIR: AlignedTable = AlignedTable {
    entries: [0; ENTRIES_PER_TABLE],
};

/// Identity page tables (NUM_PGTABS tables, each 4 KB).
static mut PGTABS: [AlignedTable; NUM_PGTABS] = [AlignedTable {
    entries: [0; ENTRIES_PER_TABLE],
}; NUM_PGTABS];

/// Physical address of the identity page directory, loaded into CR3 by
/// `__phys_memcpy` / `__phys_memset` to access physical memory without
/// disabling paging.
#[no_mangle]
pub static mut IDENTITY_CR3: usize = 0;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Initializes the identity-mapping page directory and page tables so that
/// virtual address == physical address for the range [0, MEMORY_SIZE).
///
/// After this function returns, assembly routines can load `IDENTITY_CR3`
/// into CR3 to temporarily switch to the identity address space, perform
/// physical-memory operations with paging enabled, and then restore the
/// original CR3.  Because CR3 writes are **not** intercepted by KVM on AMD
/// SVM when Nested Page Tables (NPT) are active, this eliminates the
/// VM-exit overhead that the previous CR0.PG toggle approach incurred.
///
/// # Safety
///
/// Must be called exactly once during kernel initialization, before any call
/// to `__phys_memcpy`, `__phys_memcpy32`, or `__phys_memset`.
///
pub unsafe fn init() {
    // Fill each page table with identity-mapped PTEs.
    for (i, pgtab) in PGTABS.iter_mut().enumerate().take(NUM_PGTABS) {
        for (j, entry) in pgtab.entries.iter_mut().enumerate() {
            let phys_addr: usize = (i * ENTRIES_PER_TABLE + j) * mem::PAGE_SIZE;
            if phys_addr < config::kernel::MEMORY_SIZE {
                *entry = (phys_addr as u32) | FLAGS_RW_PRESENT;
            }
        }
    }

    // Point each page directory entry at the corresponding page table.
    for (i, pgtab) in PGTABS.iter().enumerate().take(NUM_PGTABS) {
        let pgtab_phys: u32 = core::ptr::addr_of!(*pgtab) as u32;
        PGDIR.entries[i] = (pgtab_phys & 0xFFFFF000) | FLAGS_RW_PRESENT;
    }

    // Record the CR3 value for use by assembly routines.
    IDENTITY_CR3 = core::ptr::addr_of!(PGDIR) as usize;
}
