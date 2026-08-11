// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::arch;

include!("mod.spec.rs");

//==================================================================================================
// Modules
//==================================================================================================

#[path = "../../../shared/mem/mmu/page_directory.rs"]
pub mod page_directory;
#[path = "../../../shared/mem/mmu/page_table.rs"]
pub mod page_table;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[inline(never)]
pub unsafe fn load_page_directory(cr3: usize) {
    // arch::asm!(
    //     "mov {0}, %eax",
    //     "mov %eax, %cr3",
    //     "mov %cr0, %eax",
    //     "or $0x80000000, %eax",
    //     "mov %eax, %cr0",
    //     in(reg) cr3,
    //     options(nostack, att_syntax)
    // );
    unsafe {
        env_interaction_write_cr3(cr3);
    }
    let mut cr0: usize = unsafe { env_interaction_read_cr0() };
    cr0 |= ENV_INTERACTION_CR0_PG;
    unsafe {
        env_interaction_write_cr0(cr0);
    }
}
