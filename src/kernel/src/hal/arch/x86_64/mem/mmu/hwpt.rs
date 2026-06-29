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

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum number of page-table pages that can be allocated from the static pool.
const MAX_PT_PAGES: usize = 1024;

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

/// Free list of page-table page physical addresses returned by [`free_pt_page`]. Reusing freed
/// pages keeps the static pool from being exhausted by process churn (fork/exec/exit).
static mut PT_FREELIST: [u64; MAX_PT_PAGES] = [0; MAX_PT_PAGES];

/// Number of valid entries in [`PT_FREELIST`].
static mut PT_FREELIST_LEN: usize = 0;

/// Physical address of the PML4 (read from CR3 during `init()`).
static mut PML4_PADDR: usize = 0;

/// Whether the hardware page table manager has been initialized.
static mut INITIALIZED: bool = false;

//==================================================================================================
// Private Helpers
//==================================================================================================

/// Allocates a zeroed page-table page from the free list or the static pool. Returns its physical
/// address.
///
/// # Panics
///
/// Panics if the pool is exhausted.
unsafe fn alloc_pt_page() -> u64 {
    // Reuse a freed page if one is available.
    if PT_FREELIST_LEN > 0 {
        PT_FREELIST_LEN -= 1;
        let paddr: u64 = PT_FREELIST[PT_FREELIST_LEN];
        // Zero the reused page before handing it out.
        let ptr: *mut u64 = paddr as *mut u64;
        for i in 0..ENTRIES_PER_TABLE {
            core::ptr::write_volatile(ptr.add(i), 0);
        }
        return paddr;
    }

    let idx: usize = PT_POOL_NEXT;
    if idx >= MAX_PT_PAGES {
        error!("hwpt: page-table pool exhausted (used={}, max={})", idx, MAX_PT_PAGES);
        panic!("hwpt: page-table pool exhausted");
    }
    PT_POOL_NEXT += 1;
    // The pool lives in identity-mapped kernel BSS, so virtual address == physical address.
    let ptr: *const PtPage = &PT_POOL[idx];
    ptr as u64
}

/// Returns a page-table page to the free list for later reuse.
///
/// # Safety
///
/// `paddr` must be a page previously obtained from [`alloc_pt_page`] and no longer referenced by
/// any page table.
unsafe fn free_pt_page(paddr: u64) {
    if PT_FREELIST_LEN < MAX_PT_PAGES {
        PT_FREELIST[PT_FREELIST_LEN] = paddr;
        PT_FREELIST_LEN += 1;
    }
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

    // Discover boot PD0 address: PML4[0] → PDPT, PDPT[0] → PD0.
    let pml4_entry: u64 = read_entry(PML4_PADDR as u64, 0);
    let pdpt: u64 = pml4_entry & ADDR_MASK_4K;
    let pdpt_entry0: u64 = read_entry(pdpt, 0);
    BOOT_PD0_PADDR = pdpt_entry0 & ADDR_MASK_4K;

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
/// Allocates a fresh per-process 4-level page table (PML4 + PDPT) for a user address space.
///
/// The new PML4 shares the kernel's low-memory mapping by pointing `PDPT[0]` at the boot `PD0`
/// (which identity-maps `0..1 GiB`, covering all of kernel space — code, data, stacks, the IDT/GDT,
/// and the microvm control/pvclock pages). User space (`1 GiB..`) lives in `PDPT[1..]`, whose page
/// directories are created on demand by [`map_user`]. The Local APIC MMIO page (above `1 GiB`) is
/// also mapped, supervisor-only, so the kernel can issue EOIs while running on this address space
/// after an interrupt taken from user mode.
///
/// Returns the physical address of the new PML4 (the value to load into `CR3`).
///
/// # Safety
///
/// Must be called after [`init`].
pub unsafe fn create_user_pml4() -> u64 {
    assert!(INITIALIZED, "hwpt: not initialized");

    let new_pml4: u64 = alloc_pt_page();
    let new_pdpt: u64 = alloc_pt_page();

    // PDPT[0] → boot PD0 (shared kernel mapping). PTE_USER is set on this intermediate so that
    // user-accessible low pages (e.g. the pvclock page) remain reachable from Ring 3; the actual
    // U/S permission is still gated by the leaf entries inside the shared PD.
    write_entry(new_pdpt, 0, BOOT_PD0_PADDR | PTE_PRESENT | PTE_WRITABLE | PTE_USER);

    // PML4[0] → new PDPT.
    write_entry(new_pml4, 0, new_pdpt | PTE_PRESENT | PTE_WRITABLE | PTE_USER);

    // Map the Local APIC MMIO page (supervisor-only) so interrupt EOIs issued while this address
    // space is active do not fault.
    let lapic: usize = ::config::microvm::DEFAULT_LAPIC_BASE;
    map_in(new_pml4, lapic, lapic, false, true);

    new_pml4
}

/// Maps a single 4 KiB user page `vaddr` → `paddr` (User-accessible) in the given per-process PML4.
///
/// # Safety
///
/// `pml4` must be a valid PML4 physical address from [`create_user_pml4`].
pub unsafe fn map_user(pml4: u64, vaddr: usize, paddr: usize, writable: bool) {
    map_in(pml4, vaddr, paddr, true, writable);
}

/// Unmaps a single 4 KiB user page at `vaddr` in the given per-process PML4.
///
/// # Safety
///
/// `pml4` must be a valid PML4 physical address from [`create_user_pml4`].
pub unsafe fn unmap_user(pml4: u64, vaddr: usize) {
    unmap_in(pml4, vaddr);
}

/// Maps (user-accessible) a single 4 KiB page `vaddr` → `paddr` into the boot PML4's shared kernel
/// low-memory page directory (`PDPT[0]` → boot `PD0`). Because every per-process PML4 references
/// that same boot `PD0`, this makes the page visible — with the same permissions — in all address
/// spaces, mirroring how the kernel's shared identity-mapped page tables behave on 32-bit targets.
///
/// This is used by `kctrl()` to expose MMIO windows (e.g. the RAMFS) that live in low physical
/// memory to user processes. Mapping at 4 KiB granularity transparently splits the covering 2 MiB
/// identity page in boot `PD0` while preserving the surrounding supervisor-only kernel mappings.
///
/// # Safety
///
/// Must be called after [`init`]. `vaddr` must lie within the boot `PD0` coverage (the low 1 GiB).
pub unsafe fn map_kernel_mmio(vaddr: usize, paddr: usize, writable: bool) {
    assert!(INITIALIZED, "hwpt: not initialized");
    map_in(PML4_PADDR as u64, vaddr, paddr, true, writable);
}

/// Updates the writable permission of an already-mapped 4 KiB user page (used for copy-on-write).
/// If the page is not currently mapped at 4 KiB granularity, this is a no-op.
///
/// # Safety
///
/// `pml4` must be a valid PML4 physical address from [`create_user_pml4`].
pub unsafe fn protect_user(pml4: u64, vaddr: usize, writable: bool) {
    let pml4_idx: usize = (vaddr >> 39) & 0x1FF;
    let pdpt_idx: usize = (vaddr >> 30) & 0x1FF;
    let pd_idx: usize = (vaddr >> 21) & 0x1FF;
    let pt_idx: usize = (vaddr >> 12) & 0x1FF;

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
        return;
    }
    let pt: u64 = pd_entry & ADDR_MASK_4K;
    let pte: u64 = read_entry(pt, pt_idx);
    if pte & PTE_PRESENT == 0 {
        return;
    }
    let new_pte: u64 = if writable {
        pte | PTE_WRITABLE
    } else {
        pte & !PTE_WRITABLE
    };
    write_entry(pt, pt_idx, new_pte);
    invlpg(vaddr);
}

/// Tears down a per-process PML4, returning every process-owned page-table page to the free list.
///
/// The shared kernel `PD0` (referenced by `PDPT[0]`) is never freed; only the process-private
/// `PDPT[1..]` subtrees (user space plus the per-process LAPIC tables) and the `PDPT`/`PML4`
/// pages themselves are reclaimed.
///
/// # Safety
///
/// `pml4` must be a valid PML4 physical address from [`create_user_pml4`] that is no longer loaded
/// in any `CR3`.
pub unsafe fn destroy_user_pml4(pml4: u64) {
    let pml4_entry: u64 = read_entry(pml4, 0);
    if pml4_entry & PTE_PRESENT != 0 {
        let pdpt: u64 = pml4_entry & ADDR_MASK_4K;
        // Skip PDPT[0] (shared kernel PD0); free all process-private PDPT[1..] subtrees.
        for pdpt_i in 1..ENTRIES_PER_TABLE {
            let pdpt_entry: u64 = read_entry(pdpt, pdpt_i);
            if pdpt_entry & PTE_PRESENT == 0 || pdpt_entry & PDE_PS != 0 {
                continue;
            }
            let pd: u64 = pdpt_entry & ADDR_MASK_4K;
            for pd_i in 0..ENTRIES_PER_TABLE {
                let pd_entry: u64 = read_entry(pd, pd_i);
                if pd_entry & PTE_PRESENT == 0 || pd_entry & PDE_PS != 0 {
                    continue;
                }
                free_pt_page(pd_entry & ADDR_MASK_4K);
            }
            free_pt_page(pd);
        }
        free_pt_page(pdpt);
    }
    free_pt_page(pml4);
}

/// Unmaps a single 4 KiB page at `vaddr` using the given PML4.
///
/// # Safety
///
/// Caller must ensure `vaddr` is page-aligned, `pml4` is a valid PML4 physical address.
unsafe fn unmap_in(pml4: u64, vaddr: usize) {
    let pml4_idx: usize = (vaddr >> 39) & 0x1FF;
    let pdpt_idx: usize = (vaddr >> 30) & 0x1FF;
    let pd_idx: usize = (vaddr >> 21) & 0x1FF;
    let pt_idx: usize = (vaddr >> 12) & 0x1FF;

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

//==================================================================================================
// Per-Process Page Tables
//==================================================================================================

/// Physical address of the boot PD0 (supervisor-only, maps 0–1 GiB kernel space).
/// Discovered from the boot PML4 during `init()`.
static mut BOOT_PD0_PADDR: u64 = 0;

/// Maps a single 4 KiB page in a specific PML4 hierarchy.
unsafe fn map_in(pml4: u64, vaddr: usize, paddr: usize, user: bool, writable: bool) {
    let pml4_idx: usize = (vaddr >> 39) & 0x1FF;
    let pdpt_idx: usize = (vaddr >> 30) & 0x1FF;
    let pd_idx: usize = (vaddr >> 21) & 0x1FF;
    let pt_idx: usize = (vaddr >> 12) & 0x1FF;

    // Walk/create PML4 → PDPT.
    let pdpt: u64 = ensure_table(pml4, pml4_idx, user);

    // Walk/create PDPT → PD.
    let pd: u64 = ensure_table(pdpt, pdpt_idx, user);

    // Check if PD entry is a 2 MiB page (needs splitting).
    let pd_entry: u64 = read_entry(pd, pd_idx);
    let pt: u64 = if pd_entry & PTE_PRESENT != 0 && pd_entry & PDE_PS != 0 {
        let pt_addr: u64 = split_2m_entry(pd, pd_idx);
        if user {
            let new_pd_entry: u64 = read_entry(pd, pd_idx);
            if new_pd_entry & PTE_USER == 0 {
                write_entry(pd, pd_idx, new_pd_entry | PTE_USER);
            }
        }
        pt_addr
    } else if pd_entry & PTE_PRESENT != 0 {
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

    invlpg(vaddr);
}
