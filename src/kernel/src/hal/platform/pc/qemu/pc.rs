// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Shuts down the machine.
///
/// # Parameters
///
/// - `status`: The shutdown status code.
///
/// # Returns
///
/// This function never returns.
///
pub(in crate::hal::platform) fn do_shutdown(_status: usize) -> ! {
    unsafe {
        ::arch::io::out16(::config::pc::DEFAULT_VMM_PORT, ::config::pc::DEFAULT_VMM_SHUTDOWN_CMD);
    };
    loop {
        core::hint::spin_loop();
    }
}
