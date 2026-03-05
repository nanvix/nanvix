// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Hardware Page Table Manager for x86_64
//!
//! Manages the 4-level page table hierarchy (PML4 → PDPT → PD → PT)
//! that the CPU uses for virtual-to-physical address translation.
//!
//! The VMM sets up initial identity-mapped page tables with 2 MiB pages
//! covering 0–2 GiB. This module extends those tables by:
//! - Splitting 2 MiB PD entries into 4 KiB PT entries on demand.
//! - Adding new PDPT/PD/PT entries for unmapped regions (e.g., user stack).
//! - Providing `map` and `unmap` for individual 4 KiB pages.

use crate::hal::mem::FrameAddress;

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum number of page-table pages that can be allocated from the static pool.
const MAX_PT_PAGES: usize = 64;

/// Number of entries per page table level (PML4, PDPT, PD, PT).
const ENTRIES_PER_TABLE: usize = 512;

/// Page table entry flag: Present.
const PTE_PRESENT: u64 = 1 << 0;

/// Page table entry flag: Writable.
const PTE_WRITABLE: u64 = 1 << 1;

/// Page table entry flag: User-accessible.
const PTE_USER: u64 = 1 << 2;

/// Page directory entry flag: Page Size (2 MiB page).
const PDE_PS: u64 = 1 << 7;

/// Mask for extracting the physical address from a 4 KiB page table entry.
const ADDR_MASK_4K: u64 = 0x000F_FFFF_FFFF_F000;

/// Mask for extracting the physical address from a 2 MiB page directory entry.
const ADDR_MASK_2M: u64 = 0x000F_FFFF_FFE0_0000;

//==================================================================================================
// Global State
//==================================================================================================

/// Static pool of page-table pages (each 4 KiB, 512 × u64 entries).
#[repr(C, align(4096))]
struct PtPage([u64; ENTRIES_PER_TABLE]);

/// Pool of page-table pages allocated from BSS.
static mut PT_POOL: [PtPage; MAX_PT_PAGES] = {
    const ZERO_PAGE: PtPage = PtPage([0u64; ENTRIES_PER_TABLE]);
    [ZERO_PAGE; MAX_PT_PAGES]
};

/// Next free index into `PT_POOL`.
static mut PT_POOL_NEXT: usize = 0;

/// Physical address of the PML4 (read from CR3 during `init()`).
static mut PML4_PADDR: usize = 0;

/// Whether the hardware page table manager has been initialized.
static mut INITIALIZED: bool = false;

//==================================================================================================
// Private Helpers
//==================================================================================================

/// Allocates a zeroed page-table page from the static pool. Returns its physical address.
///
/// # Panics
///
/// Panics if the pool is exhausted.
unsafe fn alloc_pt_page() -> u64 {
    let idx: usize = PT_POOL_NEXT;
    assert!(idx < MAX_PT_PAGES, "hwpt: page-table pool exhausted");
    PT_POOL_NEXT += 1;
    // The pool lives in identity-mapped kernel BSS, so virtual address == physical address.
    let ptr: *const PtPage = &PT_POOL[idx];
    ptr as u64
}

/// Reads a 64-bit entry from a page table at `table_paddr[index]`.
///
/// # Safety
///
/// `table_paddr` must be a valid, identity-mapped physical address of a page table.
#[inline]
unsafe fn read_entry(table_paddr: u64, index: usize) -> u64 {
    let ptr: *const u64 = (table_paddr as usize + index * 8) as *const u64;
    core::ptr::read_volatile(ptr)
}

/// Writes a 64-bit entry to a page table at `table_paddr[index]`.
///
/// # Safety
///
/// `table_paddr` must be a valid, identity-mapped physical address of a page table.
#[inline]
unsafe fn write_entry(table_paddr: u64, index: usize, value: u64) {
    let ptr: *mut u64 = (table_paddr as usize + index * 8) as *mut u64;
    core::ptr::write_volatile(ptr, value);
}

/// Ensures an intermediate page table entry (PML4/PDPT/PD) exists and has the required flags.
/// If the entry is not present, allocates a new zeroed page and installs it.
/// If the entry exists but lacks the User bit and `user` is true, the User bit is added.
///
/// # Safety
///
/// `table_paddr` and `index` must refer to a valid page table.
unsafe fn ensure_table(table_paddr: u64, index: usize, user: bool) -> u64 {
    let entry: u64 = read_entry(table_paddr, index);
    if entry & PTE_PRESENT != 0 {
        // Entry exists. If user access is required but the entry lacks PTE_USER, upgrade it.
        // The U/S bit must be set at every level of the page table hierarchy for user-mode
        // access to succeed.
        if user && (entry & PTE_USER == 0) {
            write_entry(table_paddr, index, entry | PTE_USER);
        }
        entry & ADDR_MASK_4K
    } else {
        // Allocate and install a new table.
        let new_table: u64 = alloc_pt_page();
        let mut flags: u64 = PTE_PRESENT | PTE_WRITABLE;
        if user {
            flags |= PTE_USER;
        }
        write_entry(table_paddr, index, new_table | flags);
        new_table
    }
}

/// Splits a 2 MiB PD entry into 512 × 4 KiB PT entries, preserving the identity mapping.
///
/// # Safety
///
/// `pd_paddr` and `pd_index` must point to a valid 2 MiB PD entry.
unsafe fn split_2m_entry(pd_paddr: u64, pd_index: usize) -> u64 {
    let pd_entry: u64 = read_entry(pd_paddr, pd_index);
    let base_2m: u64 = pd_entry & ADDR_MASK_2M;
    let flags_4k: u64 = pd_entry & 0x67; // Present, Writable, User, Accessed, Dirty — drop PS.

    let pt_page: u64 = alloc_pt_page();
    for i in 0..ENTRIES_PER_TABLE {
        let pte: u64 = (base_2m + (i as u64 * 4096)) | flags_4k;
        write_entry(pt_page, i, pte);
    }

    // Replace PD entry: point to new PT, drop PS flag.
    let new_pd_entry: u64 = pt_page | (pd_entry & 0x67);
    write_entry(pd_paddr, pd_index, new_pd_entry);

    pt_page
}

/// Flushes the TLB entry for `vaddr`.
#[inline]
unsafe fn invlpg(vaddr: usize) {
    core::arch::asm!("invlpg [{}]", in(reg) vaddr, options(nostack, preserves_flags));
}

//==================================================================================================
// Public API
//==================================================================================================

/// Initializes the hardware page table manager by reading the current CR3.
///
/// Must be called once during kernel init, after the VMM page tables are in use.
///
/// # Safety
///
/// Must be called from kernel mode with identity mapping active.
pub unsafe fn init() {
    let cr3: u64;
    core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nostack, nomem));
    PML4_PADDR = (cr3 & ADDR_MASK_4K) as usize;
    INITIALIZED = true;
}

/// Maps a single 4 KiB page: `vaddr` → `paddr`.
///
/// # Parameters
///
/// - `vaddr`: Virtual address (must be 4 KiB aligned).
/// - `paddr`: Physical frame address.
/// - `user`: If true, the page is accessible from user mode (Ring 3).
/// - `writable`: If true, the page is writable.
///
/// # Safety
///
/// Caller must ensure `vaddr` is page-aligned and `paddr` is a valid physical frame.
pub unsafe fn map(vaddr: usize, paddr: usize, user: bool, writable: bool) {
    assert!(INITIALIZED, "hwpt: not initialized");

    let pml4_idx: usize = (vaddr >> 39) & 0x1FF;
    let pdpt_idx: usize = (vaddr >> 30) & 0x1FF;
    let pd_idx: usize = (vaddr >> 21) & 0x1FF;
    let pt_idx: usize = (vaddr >> 12) & 0x1FF;

    let pml4: u64 = PML4_PADDR as u64;

    // Walk/create PML4 → PDPT.
    let pdpt: u64 = ensure_table(pml4, pml4_idx, user);

    // Walk/create PDPT → PD.
    let pd: u64 = ensure_table(pdpt, pdpt_idx, user);

    // Check if PD entry is a 2 MiB page (needs splitting).
    let pd_entry: u64 = read_entry(pd, pd_idx);
    let pt: u64 = if pd_entry & PTE_PRESENT != 0 && pd_entry & PDE_PS != 0 {
        let pt_addr: u64 = split_2m_entry(pd, pd_idx);
        // After splitting, upgrade the PD entry's User bit if user access is required.
        if user {
            let new_pd_entry: u64 = read_entry(pd, pd_idx);
            if new_pd_entry & PTE_USER == 0 {
                write_entry(pd, pd_idx, new_pd_entry | PTE_USER);
            }
        }
        pt_addr
    } else if pd_entry & PTE_PRESENT != 0 {
        // Existing PT — upgrade User bit on the PD entry if needed.
        if user && (pd_entry & PTE_USER == 0) {
            write_entry(pd, pd_idx, pd_entry | PTE_USER);
        }
        pd_entry & ADDR_MASK_4K
    } else {
        ensure_table(pd, pd_idx, user)
    };

    // Build the PT entry.
    let mut flags: u64 = PTE_PRESENT;
    if writable {
        flags |= PTE_WRITABLE;
    }
    if user {
        flags |= PTE_USER;
    }
    let pte: u64 = (paddr as u64 & ADDR_MASK_4K) | flags;
    write_entry(pt, pt_idx, pte);

    // Flush TLB for this address.
    invlpg(vaddr);
}

/// Maps a single 4 KiB page for user space: `vaddr` → `paddr` with User + Writable flags.
///
/// This is a convenience wrapper around [`map()`] for the common case.
pub unsafe fn map_user(vaddr: usize, paddr: FrameAddress) {
    map(vaddr, paddr.into_raw_value(), true, true);
}

/// Unmaps a single 4 KiB page at `vaddr`.
///
/// # Safety
///
/// Caller must ensure `vaddr` is page-aligned and currently mapped.
pub unsafe fn unmap(vaddr: usize) {
    assert!(INITIALIZED, "hwpt: not initialized");

    let pml4_idx: usize = (vaddr >> 39) & 0x1FF;
    let pdpt_idx: usize = (vaddr >> 30) & 0x1FF;
    let pd_idx: usize = (vaddr >> 21) & 0x1FF;
    let pt_idx: usize = (vaddr >> 12) & 0x1FF;

    let pml4: u64 = PML4_PADDR as u64;

    // Walk the hierarchy — if any level is missing, the page was never mapped.
    let pml4_entry: u64 = read_entry(pml4, pml4_idx);
    if pml4_entry & PTE_PRESENT == 0 {
        return;
    }
    let pdpt: u64 = pml4_entry & ADDR_MASK_4K;

    let pdpt_entry: u64 = read_entry(pdpt, pdpt_idx);
    if pdpt_entry & PTE_PRESENT == 0 {
        return;
    }
    let pd: u64 = pdpt_entry & ADDR_MASK_4K;

    let pd_entry: u64 = read_entry(pd, pd_idx);
    if pd_entry & PTE_PRESENT == 0 || pd_entry & PDE_PS != 0 {
        // Not present or still a 2 MiB page — nothing to unmap at 4 KiB granularity.
        return;
    }
    let pt: u64 = pd_entry & ADDR_MASK_4K;

    // Clear the PT entry.
    write_entry(pt, pt_idx, 0);
    invlpg(vaddr);
}
