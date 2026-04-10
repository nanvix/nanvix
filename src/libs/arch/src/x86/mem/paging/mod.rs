// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.
//==================================================================================================
// Modules
//==================================================================================================

mod flags;
mod frame;
mod pde;
mod pte;

//==================================================================================================
// Exports
//==================================================================================================

pub use flags::*;
pub use frame::FrameNumber;
pub use pde::*;
pub use pte::*;

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
