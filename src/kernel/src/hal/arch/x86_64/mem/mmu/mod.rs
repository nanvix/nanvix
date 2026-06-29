// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

pub mod hwpt;
#[path = "../../../shared/mem/mmu/page_directory.rs"]
pub mod page_directory;
#[path = "../../../shared/mem/mmu/page_table.rs"]
pub mod page_table;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Loads the top-level page table into CR3.
///
/// # Note
///
/// On x86_64, this function is intentionally a no-op. Address-space switching is handled
/// by callers via `#[cfg(target_arch = "x86_64")]` paths that write CR3 directly (see
/// `Vmem::load()`) or allocate per-process PML4s through [`hwpt::create_user_pml4()`].
/// The shared virtual-memory layer gates its calls to this function behind
/// `#[cfg(not(target_arch = "x86_64"))]`, so this code path is never reached at runtime.
///
/// # Safety
///
/// This function is unsafe because on other architectures it modifies the CR3 register.
///
#[inline(never)]
#[allow(dead_code)]
pub unsafe fn load_page_directory(_cr3: usize) {
    // No-op on x86_64: callers use #[cfg]-gated paths for CR3 management.
}
