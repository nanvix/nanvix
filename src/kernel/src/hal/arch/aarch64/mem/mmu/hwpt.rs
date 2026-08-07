// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! AArch64 stage-1 translation-table manager.
//!
//! Nanvix keeps its compact two-level software page map for bookkeeping. This module mirrors those
//! mappings into the three-level, 4-KiB-granule tables consumed by EL1 through `TTBR0_EL1`.

//==================================================================================================
// Constants
//==================================================================================================

const MAX_PT_PAGES: usize = 1024;
const ENTRIES_PER_TABLE: usize = 512;
const PAGE_SIZE: usize = 4096;

const DESC_VALID: u64 = 1 << 0;
const DESC_TABLE_OR_PAGE: u64 = 1 << 1;
const DESC_ATTR_NORMAL: u64 = 0 << 2;
const DESC_ATTR_DEVICE: u64 = 1 << 2;
const DESC_AP_USER_RW: u64 = 1 << 6;
const DESC_AP_RO: u64 = 1 << 7;
const DESC_SH_INNER: u64 = 0b11 << 8;
const DESC_AF: u64 = 1 << 10;
const DESC_NOT_GLOBAL: u64 = 1 << 11;
const DESC_PXN: u64 = 1 << 53;
const DESC_UXN: u64 = 1 << 54;
const TABLE_ADDR_MASK: u64 = 0x0000_FFFF_FFFF_F000;

//==================================================================================================
// Global State
//==================================================================================================

#[derive(Clone, Copy)]
#[repr(C, align(4096))]
struct PtPage([u64; ENTRIES_PER_TABLE]);

static mut PT_POOL: [PtPage; MAX_PT_PAGES] = {
    const ZERO_PAGE: PtPage = PtPage([0; ENTRIES_PER_TABLE]);
    [ZERO_PAGE; MAX_PT_PAGES]
};
static mut PT_POOL_NEXT: usize = 0;
static mut PT_FREELIST: [u64; MAX_PT_PAGES] = [0; MAX_PT_PAGES];
static mut PT_FREELIST_LEN: usize = 0;
static mut KERNEL_ROOT: u64 = 0;
static mut INITIALIZED: bool = false;

//==================================================================================================
// Private Helpers
//==================================================================================================

unsafe fn alloc_pt_page() -> u64 {
    if PT_FREELIST_LEN != 0 {
        PT_FREELIST_LEN -= 1;
        let page: u64 = PT_FREELIST[PT_FREELIST_LEN];
        core::ptr::write_bytes(page as *mut u8, 0, PAGE_SIZE);
        return page;
    }

    let index: usize = PT_POOL_NEXT;
    if index >= MAX_PT_PAGES {
        panic!("AArch64 translation-table pool exhausted");
    }
    PT_POOL_NEXT += 1;
    (&raw const PT_POOL[index]) as u64
}

unsafe fn free_pt_page(page: u64) {
    if PT_FREELIST_LEN < MAX_PT_PAGES {
        PT_FREELIST[PT_FREELIST_LEN] = page;
        PT_FREELIST_LEN += 1;
    }
}

#[inline]
unsafe fn read_entry(table: u64, index: usize) -> u64 {
    core::ptr::read_volatile((table as usize + index * 8) as *const u64)
}

#[inline]
unsafe fn write_entry(table: u64, index: usize, value: u64) {
    core::ptr::write_volatile((table as usize + index * 8) as *mut u64, value);
}

unsafe fn ensure_table(table: u64, index: usize) -> u64 {
    let entry: u64 = read_entry(table, index);
    if entry & DESC_VALID != 0 {
        return entry & TABLE_ADDR_MASK;
    }

    let child: u64 = alloc_pt_page();
    write_entry(table, index, child | DESC_VALID | DESC_TABLE_OR_PAGE);
    child
}

unsafe fn map_in(root: u64, vaddr: usize, paddr: usize, user: bool, writable: bool, device: bool) {
    let l1_index: usize = (vaddr >> 30) & 0x1ff;
    let l2_index: usize = (vaddr >> 21) & 0x1ff;
    let l3_index: usize = (vaddr >> 12) & 0x1ff;

    let l2: u64 = ensure_table(root, l1_index);
    let l3: u64 = ensure_table(l2, l2_index);

    let mut descriptor: u64 = (paddr as u64 & TABLE_ADDR_MASK)
        | DESC_VALID
        | DESC_TABLE_OR_PAGE
        | DESC_AF
        | DESC_SH_INNER;
    descriptor |= if device {
        DESC_ATTR_DEVICE | DESC_PXN | DESC_UXN
    } else {
        DESC_ATTR_NORMAL
    };
    if user {
        descriptor |= DESC_AP_USER_RW | DESC_NOT_GLOBAL | DESC_PXN;
        if !writable {
            descriptor |= DESC_AP_RO;
        }
    } else {
        descriptor |= DESC_UXN;
        if !writable {
            descriptor |= DESC_AP_RO;
        }
    }

    let old_descriptor: u64 = read_entry(l3, l3_index);
    if old_descriptor == descriptor {
        return;
    }

    // Replacing a live translation with a different output address or memory type requires Arm's
    // break-before-make sequence. The public mapping helpers perform the final invalidate after
    // installing the new descriptor; invalidate the old descriptor here before replacing it.
    if INITIALIZED && old_descriptor & DESC_VALID != 0 {
        write_entry(l3, l3_index, 0);
        invalidate(vaddr);
    }

    write_entry(l3, l3_index, descriptor);
}

unsafe fn unmap_in(root: u64, vaddr: usize) {
    let l1_index: usize = (vaddr >> 30) & 0x1ff;
    let l2_index: usize = (vaddr >> 21) & 0x1ff;
    let l3_index: usize = (vaddr >> 12) & 0x1ff;

    let l1: u64 = read_entry(root, l1_index);
    if l1 & DESC_VALID == 0 {
        return;
    }
    let l2: u64 = l1 & TABLE_ADDR_MASK;
    let l2_entry: u64 = read_entry(l2, l2_index);
    if l2_entry & DESC_VALID == 0 {
        return;
    }
    let l3: u64 = l2_entry & TABLE_ADDR_MASK;
    write_entry(l3, l3_index, 0);
    invalidate(vaddr);
}

unsafe fn invalidate(vaddr: usize) {
    let operand: usize = vaddr >> 12;
    core::arch::asm!(
        "dsb ishst",
        "tlbi vaae1is, {operand}",
        "dsb ish",
        "isb",
        operand = in(reg) operand,
        options(nostack, preserves_flags),
    );
}

unsafe fn map_range(root: u64, base: usize, size: usize, device: bool) {
    let end: usize = base
        .checked_add(size)
        .expect("translation-table range overflow");
    let mut address: usize = base;
    while address < end {
        map_in(root, address, address, false, true, device);
        address += PAGE_SIZE;
    }
}

//==================================================================================================
// Public API
//==================================================================================================

/// Builds the kernel identity map and enables the EL1 MMU.
pub unsafe fn init() {
    if INITIALIZED {
        return;
    }

    let root: u64 = alloc_pt_page();
    map_range(root, 0, ::config::kernel::MEMORY_SIZE, false);

    map_range(root, ::config::microvm::DEFAULT_GICD_BASE, 0x1_0000, true);
    map_range(root, ::config::microvm::DEFAULT_GICR_BASE, 0x2_0000, true);
    map_range(root, ::config::microvm::DEFAULT_GITS_BASE, 0x2_0000, true);
    map_range(root, ::config::microvm::DEFAULT_MMIO_DOORBELL_BASE, PAGE_SIZE, true);

    // Attr0: normal write-back, read/write allocate. Attr1: device-nGnRE.
    let mair: u64 = 0xff | (0x04 << 8);
    // 39-bit TTBR0 VA, 4-KiB granule, inner-shareable, WBWA walks, 32-bit PA, TTBR1 disabled.
    let tcr: u64 = 25 | (0b01 << 8) | (0b01 << 10) | (0b11 << 12) | (1 << 23);
    core::arch::asm!(
        "msr mair_el1, {mair}",
        "msr tcr_el1, {tcr}",
        "msr ttbr0_el1, {root}",
        "dsb ish",
        "isb",
        mair = in(reg) mair,
        tcr = in(reg) tcr,
        root = in(reg) root,
        options(nostack, preserves_flags),
    );

    let mut sctlr: u64;
    core::arch::asm!("mrs {sctlr}, sctlr_el1", sctlr = out(reg) sctlr, options(nostack));
    // Enable the MMU and caches.
    sctlr |= (1 << 0) | (1 << 2) | (1 << 12);
    core::arch::asm!(
        "msr sctlr_el1, {sctlr}",
        "isb",
        sctlr = in(reg) sctlr,
        options(nostack, preserves_flags),
    );

    KERNEL_ROOT = root;
    INITIALIZED = true;
}

/// Returns the kernel `TTBR0_EL1` root.
pub unsafe fn kernel_root() -> u64 {
    assert!(INITIALIZED, "AArch64 translation tables are not initialized");
    KERNEL_ROOT
}

/// Creates a process translation root sharing the kernel's low 1-GiB mapping.
pub unsafe fn create_user_pml4() -> u64 {
    assert!(INITIALIZED, "AArch64 translation tables are not initialized");
    let root: u64 = alloc_pt_page();
    write_entry(root, 0, read_entry(KERNEL_ROOT, 0));
    root
}

pub unsafe fn map_user(root: u64, vaddr: usize, paddr: usize, writable: bool) {
    map_in(root, vaddr, paddr, true, writable, false);
    invalidate(vaddr);
}

pub unsafe fn unmap_user(root: u64, vaddr: usize) {
    unmap_in(root, vaddr);
}

pub unsafe fn protect_user(root: u64, vaddr: usize, writable: bool) {
    let l1: u64 = read_entry(root, (vaddr >> 30) & 0x1ff);
    if l1 & DESC_VALID == 0 {
        return;
    }
    let l2: u64 = read_entry(l1 & TABLE_ADDR_MASK, (vaddr >> 21) & 0x1ff);
    if l2 & DESC_VALID == 0 {
        return;
    }
    let l3: u64 = l2 & TABLE_ADDR_MASK;
    let index: usize = (vaddr >> 12) & 0x1ff;
    let entry: u64 = read_entry(l3, index);
    if entry & DESC_VALID == 0 {
        return;
    }
    let entry: u64 = if writable {
        entry & !DESC_AP_RO
    } else {
        entry | DESC_AP_RO
    };
    let old_entry: u64 = read_entry(l3, index);
    if old_entry == entry {
        return;
    }

    // Use break-before-make for permission changes as well. This is stricter than required for
    // every AP-bit transition, but keeps all live descriptor replacements on one safe path.
    write_entry(l3, index, 0);
    invalidate(vaddr);
    write_entry(l3, index, entry);
    invalidate(vaddr);
}

pub unsafe fn map_kernel_mmio(vaddr: usize, paddr: usize, writable: bool) {
    map_in(KERNEL_ROOT, vaddr, paddr, true, writable, true);
    invalidate(vaddr);
}

pub unsafe fn destroy_user_pml4(root: u64) {
    for l1_index in 1..ENTRIES_PER_TABLE {
        let l1: u64 = read_entry(root, l1_index);
        if l1 & DESC_VALID == 0 {
            continue;
        }
        let l2: u64 = l1 & TABLE_ADDR_MASK;
        for l2_index in 0..ENTRIES_PER_TABLE {
            let l2_entry: u64 = read_entry(l2, l2_index);
            if l2_entry & DESC_VALID != 0 {
                free_pt_page(l2_entry & TABLE_ADDR_MASK);
            }
        }
        free_pt_page(l2);
    }
    free_pt_page(root);
}
