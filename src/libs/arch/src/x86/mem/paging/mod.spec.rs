// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// Equivalent to the replaced instruction because it invalidates the same virtual page.
unsafe fn env_interaction_invalidate_tlb_page(vaddr: usize) {
    unsafe {
        ::core::arch::asm!(
            "invlpg ({0})",
            in(reg) vaddr,
            options(nostack, preserves_flags, att_syntax)
        );
    }
}
