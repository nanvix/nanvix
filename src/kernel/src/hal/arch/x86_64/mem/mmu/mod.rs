// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::arch;

//==================================================================================================
// Modules
//==================================================================================================

pub mod page_directory;
pub mod page_table;
pub mod pdpt;
pub mod pml4;

//==================================================================================================
// Exports
//==================================================================================================

#[allow(unused_imports)]
pub use pdpt::Pdpt;
#[allow(unused_imports)]
pub use pml4::Pml4;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Loads a PML4 table address into CR3.
///
/// In x86_64 long mode, paging is always enabled. Writing CR3 reloads the page tables.
#[inline(never)]
pub unsafe fn load_pml4(cr3: usize) {
    arch::asm!(
        "mov cr3, {}",
        in(reg) cr3,
        options(nostack)
    );
}

/// Loads a PML4 table address into CR3.
///
/// This is an alias for [`load_pml4`] kept for backward compatibility.
#[inline(always)]
pub unsafe fn load_page_directory(cr3: usize) {
    load_pml4(cr3);
}
