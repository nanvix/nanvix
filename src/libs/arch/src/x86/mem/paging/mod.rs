// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.
//==================================================================================================
// Modules
//==================================================================================================

use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
include!("mod.spec.rs");
#[cfg(verus_keep_ghost)]
include!("mod.proof.rs");

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
// Trust boundary (see `verus-ai-logs/tcb-allowed.md` and the module's
// `verus-unsupported.md`): the body is a single `core::arch::asm!` block issuing the
// `invlpg` instruction, which flushes the CPU TLB entry for `vaddr`. Verus does not
// support inline-asm expressions, so the body cannot be verified. The effect is purely
// on hardware TLB state — outside Verus' memory model and invisible to every caller's
// Rust-visible state — so the faithful contract is empty (no `requires`, trivial
// `ensures`): any `usize` is accepted, no error path, and no Rust-visible state changes,
// hence every caller-side invariant is preserved. This matches the inherited upstream
// `assume_specification[ ::arch::mem::paging::invlpg ]` (no `requires`/`ensures`).
#[inline]
#[verus_verify(external_body)]
pub unsafe fn invlpg(vaddr: usize) {
    core::arch::asm!(
        "invlpg ({0})",
        in(reg) vaddr,
        options(nostack, preserves_flags, att_syntax)
    );
}
