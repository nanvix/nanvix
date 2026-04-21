// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Guest virtual address translation via manual page-table walk.
//!
//! Nanvix is a 32-bit x86 OS using two-level paging (PD + PT).
//! The host can read guest physical memory directly through the
//! mapped vmem buffer, so no WHP API call is needed.

//==================================================================================================
// Imports
//==================================================================================================

use ::arch::mem::paging::{
    self,
    PageDirectoryEntry,
    PageSizeFlag,
    PageTableEntry,
    PresentFlag,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Mask for the page offset within a 4 KiB page.
#[allow(clippy::cast_possible_truncation)] // 32-bit guest constants.
const PAGE_OFFSET_MASK: u32 = (::arch::mem::PAGE_SIZE as u32) - 1;
/// Mask for extracting the physical frame address from a PDE/PTE.
const FRAME_MASK: u32 = !(PAGE_OFFSET_MASK);
/// Mask for the large page (4 MiB) frame address.
#[allow(clippy::cast_possible_truncation)] // 32-bit guest constants.
const LARGE_PAGE_FRAME_MASK: u32 = !(::arch::mem::PGTAB_SIZE as u32 - 1);
/// Mask for the offset within a 4 MiB large page.
#[allow(clippy::cast_possible_truncation)] // 32-bit guest constants.
const LARGE_PAGE_OFFSET_MASK: u32 = ::arch::mem::PGTAB_SIZE as u32 - 1;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Translates a guest virtual address to a guest physical address
/// by walking the guest's two-level page tables.
///
/// # Parameters
///
/// - `vmem_ptr`: Host pointer to the start of guest physical memory.
/// - `vmem_size`: Total size of guest physical memory in bytes.
/// - `cr3`: Guest CR3 register value (physical address of the page directory).
/// - `gva`: Guest virtual address to translate.
///
/// # Returns
///
/// `Some(gpa)` if the translation succeeds, `None` if any entry is not present
/// or the address is out of bounds.
pub fn translate_gva(vmem_ptr: *const u8, vmem_size: usize, cr3: u32, gva: u32) -> Option<u32> {
    let pd_base: usize = (cr3 & FRAME_MASK) as usize;
    let pd_index: usize = paging::pd_index(gva as usize).into_raw();
    let pt_index: usize = paging::pt_index(gva as usize).into_raw();
    let offset: u32 = gva & PAGE_OFFSET_MASK;

    // Read PDE.
    let pde_addr: usize = pd_index
        .checked_mul(PageDirectoryEntry::SIZE)
        .and_then(|v| pd_base.checked_add(v))?;
    if pde_addr + PageDirectoryEntry::SIZE > vmem_size {
        return None;
    }
    let pde: u32 = unsafe { vmem_ptr.add(pde_addr).cast::<u32>().read_unaligned() };
    if !PresentFlag::is_set(pde) {
        return None;
    }

    // Check for 4 MiB large page (PS bit).
    if matches!(PageSizeFlag::from_raw_value(pde), PageSizeFlag::Large) {
        let frame: u32 = pde & LARGE_PAGE_FRAME_MASK;
        let large_offset: u32 = gva & LARGE_PAGE_OFFSET_MASK;
        return Some(frame | large_offset);
    }

    // Read PTE from the page table.
    let pt_base: usize = (pde & FRAME_MASK) as usize;
    let pte_addr: usize = pt_index
        .checked_mul(PageTableEntry::SIZE)
        .and_then(|v| pt_base.checked_add(v))?;
    if pte_addr + PageTableEntry::SIZE > vmem_size {
        return None;
    }
    let pte: u32 = unsafe { vmem_ptr.add(pte_addr).cast::<u32>().read_unaligned() };
    if !PresentFlag::is_set(pte) {
        return None;
    }

    let frame: u32 = pte & FRAME_MASK;
    Some(frame | offset)
}

/// Reads a u32 from guest physical memory.
///
/// Returns `None` if the address is out of bounds.
#[inline]
pub fn read_gpa_u32(vmem_ptr: *const u8, vmem_size: usize, gpa: u32) -> Option<u32> {
    let addr: usize = gpa as usize;
    if addr + core::mem::size_of::<u32>() > vmem_size {
        return None;
    }
    Some(unsafe { vmem_ptr.add(addr).cast::<u32>().read_unaligned() })
}
