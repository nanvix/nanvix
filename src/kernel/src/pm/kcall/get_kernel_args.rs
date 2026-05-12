// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Global State
//==================================================================================================

/// Kernel arguments string, set once during boot.
static mut KERNEL_ARGS: &str = "";

///
/// # Description
///
/// Stores the kernel arguments string during boot.
///
/// # Safety
///
/// Must be called exactly once during boot, before any user process is started.
///
pub unsafe fn set_kernel_args(args: &'static str) {
    // SAFETY: called once during single-threaded boot.
    unsafe {
        KERNEL_ARGS = args;
    }
}
