// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.
//==================================================================================================
// Modules
//==================================================================================================

mod flags;
mod frame;
mod pde;
mod pte;
mod table;

//==================================================================================================
// Exports
//==================================================================================================

pub use flags::*;
pub use frame::FrameNumber;
pub use pde::*;
pub use pte::*;
pub use table::*;

// Re-export x86_64-specific paging types so they are accessible at `arch::mem::paging::*`.
#[cfg(target_arch = "x86_64")]
pub use crate::x86_64::mem::paging::*;

//==================================================================================================
// Types
//==================================================================================================

///
/// # Description
///
/// Word type for page table entries.
///
#[cfg(target_arch = "x86")]
pub type PteWord = u32;
/// Word type for page table entries.
#[cfg(target_arch = "x86_64")]
pub type PteWord = u64;

///
/// # Description
///
/// Log2 of the size of [`PteWord`] in bytes.
///
pub const PTE_WORD_SIZE_LOG2: usize = ::core::mem::size_of::<PteWord>().trailing_zeros() as usize;

///
/// # Description
///
/// Mask for extracting the physical address from a 4 KiB-aligned page table entry.
///
/// On x86: bits 12–31. On x86_64: bits 12–51 (reserved/NX bits masked out).
///
#[cfg(target_arch = "x86")]
pub const PHYS_ADDR_MASK: PteWord = 0xFFFFF000;
/// Mask for extracting the physical address from a 4 KiB-aligned page table entry.
#[cfg(target_arch = "x86_64")]
pub const PHYS_ADDR_MASK: PteWord = 0x000F_FFFF_FFFF_F000;

///
/// # Description
///
/// Mask for extracting the physical base address from a large page entry.
///
/// On x86 (PSE 4 MiB): bits 22–31. On x86_64 (2 MiB): bits 21–51.
///
#[cfg(target_arch = "x86")]
pub const LARGE_PAGE_ADDR_MASK: PteWord = 0xFFC00000;
/// Mask for extracting the physical base address from a large page entry.
#[cfg(target_arch = "x86_64")]
pub const LARGE_PAGE_ADDR_MASK: PteWord = 0x000F_FFFF_FFE0_0000;

///
/// # Description
///
/// Number of page-sized pages reserved for the root paging hierarchy.
///
/// On 32-bit x86 with non-PAE paging, this corresponds to one page directory.
/// On x86_64, the hierarchy includes PML4, PDPT, and PDs.
///
#[cfg(target_arch = "x86")]
pub const NUM_HIERARCHY_PAGES: usize = 1;
/// Number of page-sized pages reserved for the root paging hierarchy.
#[cfg(target_arch = "x86_64")]
pub const NUM_HIERARCHY_PAGES: usize = 4;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Flushes the TLB entry for the page containing `vaddr`.
///
/// # Safety
///
/// Must be called from kernel mode (ring 0).
///
#[inline]
pub unsafe fn invlpg(vaddr: usize) {
    #[cfg(target_arch = "x86")]
    core::arch::asm!(
        "invlpg ({0})",
        in(reg) vaddr,
        options(nostack, preserves_flags, att_syntax)
    );
    #[cfg(target_arch = "x86_64")]
    core::arch::asm!(
        "invlpg [{0}]",
        in(reg) vaddr,
        options(nostack, preserves_flags)
    );
}
