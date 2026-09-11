// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#[cfg(any(verus_keep_ghost, verus_keep_ghost_body))]
use crate::mm::PageDirectoryStorage;
#[cfg(any(verus_keep_ghost, verus_keep_ghost_body))]
use self::page_directory::PageDirectory;

/// Paging-enable bit in CR0.
const ENV_INTERACTION_CR0_PG: usize = 1 << 31;

verus! {

/// Returns whether `cr3` names the supplied initialized x86 page-directory root.
pub open spec fn valid_cr3_root(
    cr3: usize,
    root: &PageDirectory<PageDirectoryStorage>,
) -> bool {
    &&& root.ready_for_mmu()
    &&& root.physical_base() == cr3 as int
}

} // verus!

// Equivalent to the replaced instruction because it installs the same value in CR3.
#[verus_verify(external_body)]
#[verus_spec(
    with
        Ghost(root): Ghost<&PageDirectory<PageDirectoryStorage>>,
    requires
        valid_cr3_root(cr3, root),
)]
unsafe fn env_interaction_write_cr3(cr3: usize) {
    unsafe {
        arch::asm!(
            "mov {0}, %eax",
            "mov %eax, %cr3",
            in(reg) cr3,
            options(nostack, att_syntax)
        );
    }
}

// Equivalent to the replaced instruction because it returns the current CR0 value.
unsafe fn env_interaction_read_cr0() -> usize {
    let cr0: usize;
    unsafe {
        arch::asm!("mov %cr0, {0}", out(reg) cr0, options(nostack, att_syntax));
    }
    cr0
}

// Equivalent to the replaced instruction because it installs the same value in CR0.
unsafe fn env_interaction_write_cr0(cr0: usize) {
    unsafe {
        arch::asm!("mov {0}, %cr0", in(reg) cr0, options(nostack, att_syntax));
    }
}
