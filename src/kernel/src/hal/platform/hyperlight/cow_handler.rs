// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::arch::mem;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Safety-net CoW page fault handler.
///
/// After eager pre-faulting in `hyperlight_pre_kmain()`, no CoW page faults should occur during
/// normal kernel execution. If one does, it indicates a bug (a page missed by the pre-fault walk).
/// This handler returns `false` to let the normal exception path handle (and report) the fault.
///
/// # Parameters
///
/// - `fault_addr`:  The faulting virtual address (from CR2).
/// - `error_code`:  The hardware page-fault error code.
///
/// # Returns
///
/// Always returns `false`.
///
#[unsafe(no_mangle)]
pub extern "C" fn try_handle_cow_page_fault(_fault_addr: u32, _error_code: u32) -> bool {
    // After eager pre-faulting, no CoW faults should occur.
    // Log and let the normal exception path handle it.
    false
}

/// Allocates a single frame from the scratch bump allocator.
///
/// Reads the current bump pointer from the scratch metadata slot and advances it by one page.
pub(super) fn bump_alloc_frame() -> u32 {
    let alloc_ptr = ::hyperlight_guest::layout::allocator_gva();
    let gpa: u64 = unsafe { core::ptr::read_volatile(alloc_ptr) };
    unsafe {
        core::ptr::write_volatile(alloc_ptr, gpa + mem::PAGE_SIZE as u64);
    }
    gpa as u32
}
