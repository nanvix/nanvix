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

#[inline(never)]
pub unsafe fn load_page_directory(cr3: usize) {
    // Write CR3 to load the new page directory (this flushes the TLB).
    // Then, enable paging (CR0.PG) only if it is not already enabled.
    //
    // On AMD SVM, **every** CR0 write causes an unconditional VM exit
    // (INTERCEPT_CR0_WRITE), even if the written value does not change.
    // By skipping the CR0 write when PG is already set, we eliminate that
    // overhead for all calls after the initial paging-enable during boot.
    arch::asm!(
        "mov {0}, %eax",
        "mov %eax, %cr3",
        "mov %cr0, %eax",
        "test $0x80000000, %eax",
        "jnz 1f",
        "or $0x80000000, %eax",
        "mov %eax, %cr0",
        "1:",
        in(reg) cr3,
        options(nostack, att_syntax)
    );
}
