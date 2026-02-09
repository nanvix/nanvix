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

///
/// # Description
///
/// Invalidates the TLB entry for a single page using the `invlpg` instruction.
///
/// This is much cheaper than a full TLB flush (CR3 reload) because it only
/// invalidates the single TLB entry for the given virtual address, rather than
/// flushing all entries. Under nested virtualization, `invlpg` does not cause
/// a VM exit (KVM handles it in-guest via NPT), unlike CR3 writes which flush
/// the entire TLB and trigger expensive nested page table walks for every
/// subsequent memory access.
///
/// # Parameters
///
/// - `vaddr`: The virtual address of the page whose TLB entry should be
///   invalidated.
///
/// # Safety
///
/// The caller must ensure that `vaddr` is a valid virtual address.
///
#[inline(always)]
pub unsafe fn invalidate_page(vaddr: usize) {
    arch::asm!(
        "invlpg ({0})",
        in(reg) vaddr,
        options(nostack, att_syntax)
    );
}
