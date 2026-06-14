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

#[cfg(verus_keep_ghost)]
pub use frame::spec_max_frame_number;
pub use pde::*;
pub use pte::*;
pub use table::*;

//==================================================================================================
// Types
//==================================================================================================

///
/// # Description
///
/// Word type for page table entries (32-bit on x86).
///
pub type PteWord = u32;

///
/// # Description
///
/// Log2 of the size of [`PteWord`] in bytes.
///
pub const PTE_WORD_SIZE_LOG2: usize = ::core::mem::size_of::<PteWord>().trailing_zeros() as usize;

///
/// # Description
///
/// Number of page-sized pages reserved for the root paging hierarchy on x86.
///
/// On 32-bit x86 with non-PAE paging, this corresponds to one page directory.
///
pub const NUM_HIERARCHY_PAGES: usize = 1;

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
    core::arch::asm!(
        "invlpg ({0})",
        in(reg) vaddr,
        options(nostack, preserves_flags, att_syntax)
    );
}
