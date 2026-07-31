// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

pub mod hwpt;
#[path = "../../shared/mem/mmu/page_directory.rs"]
pub mod page_directory;
#[path = "../../shared/mem/mmu/page_table.rs"]
pub mod page_table;

/// Loads an EL1 translation-table root.
///
/// # Safety
///
/// `root` must identify a valid stage-1 translation table.
pub unsafe fn load_page_directory(root: usize) {
    core::arch::asm!(
        "msr ttbr0_el1, {root}",
        "dsb ish",
        "tlbi vmalle1is",
        "dsb ish",
        "isb",
        root = in(reg) root,
        options(nostack, preserves_flags),
    );
}
