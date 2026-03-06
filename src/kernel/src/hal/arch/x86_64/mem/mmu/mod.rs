// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

pub mod hwpt;
pub mod page_directory;
pub mod page_table;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Loads the PML4 (top-level page table) into CR3.
///
/// # Safety
///
/// This function is unsafe because it modifies the CR3 register, which controls
/// the page table hierarchy used by the processor for virtual address translation.
///
/// # Note
///
/// On x86_64, the kernel currently relies on the VMM-provided identity-mapped page tables
/// (PML4 → PDPT → PD with 2 MiB pages covering the first 2 GiB). The kernel's own paging
/// structures are 32-bit (2-level) and incompatible with long mode's 4-level paging. The
/// [`hwpt`] module extends the VMM tables with 4 KiB mappings for user pages, but the
/// PML4 root stays the same. This function is intentionally a no-op.
///
#[inline(never)]
#[allow(dead_code)]
pub unsafe fn load_page_directory(_cr3: usize) {
    // Intentionally a no-op: CR3 is managed per-process via hwpt::alloc_process_pml4().
}
