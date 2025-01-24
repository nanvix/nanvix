// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Shutdowns the machine.
///
/// # Return
///
/// This function never returns.
///
pub fn shutdown() -> ! {
    unsafe {
        ::sys::arch::io::out16(::config::pc::DEFAULT_VMM_PORT, ::config::pc::DEFAULT_VMM_SHUTDOWN_CMD);
    };
    loop {
        core::hint::spin_loop();
    }
}
