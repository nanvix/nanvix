// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::kargs;

//==================================================================================================
// Constants
//==================================================================================================

/// Magic value used for in-kernel verification of the kernel arguments mechanism.
const TEST_KERNEL_ARGS_MAGIC: &str = "test_magic=0xDEADBEEF";

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Runs the in-kernel tests of the memory, kernel heap, clock, and event subsystems.
///
pub(crate) fn test() {
    if !crate::hal::mem::test() {
        panic!("memory tests failed");
    }
    if !crate::mm::test() {
        panic!("kernel heap tests failed");
    }
    #[cfg(feature = "microvm")]
    if !crate::hal::platform::pvclock::test() {
        panic!("pvclock tests failed");
    }
    if !crate::event::test() {
        panic!("event manager tests failed");
    }
}

///
/// # Description
///
/// Verifies that the kernel received the expected magic kernel arguments from the command line.
/// The test TOML configs pass `test_magic=0xDEADBEEF` as a kernel argument, so by the time this
/// function runs the stored value must match [`TEST_KERNEL_ARGS_MAGIC`].
///
/// # Parameters
///
/// - `kernel_args`: The kernel arguments string parsed from boot info.
///
pub(crate) fn test_kernel_args(kernel_args: &str) {
    info!("testing kernel arguments...");

    assert!(
        !kernel_args.is_empty(),
        "kernel args: expected non-empty string, got empty (was --kernel-args passed?)",
    );

    assert!(
        kernel_args == TEST_KERNEL_ARGS_MAGIC,
        "kernel args: expected={:?}, got={:?}",
        TEST_KERNEL_ARGS_MAGIC,
        kernel_args,
    );

    // Verify that the getter returns the same value that was stored.
    let stored = kargs::get_kernel_args();
    assert!(
        stored == kernel_args,
        "get_kernel_args() mismatch: expected={:?}, got={:?}",
        kernel_args,
        stored,
    );

    info!("kernel arguments test passed");
}
