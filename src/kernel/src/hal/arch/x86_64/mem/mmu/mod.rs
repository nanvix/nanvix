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

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Loads a PML4 table address into CR3.
///
/// In x86_64 long mode, paging is always enabled. Writing CR3 reloads the page tables.
#[inline(never)]
pub unsafe fn load_page_directory(cr3: usize) {
    arch::asm!(
        "mov cr3, {}",
        in(reg) cr3,
        options(nostack)
    );
}
