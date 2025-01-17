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
        ::sys::arch::io::out16(::config::hal::DEFAULT_VMM_PORT, 0x2000);
    };
    loop {
        core::hint::spin_loop();
    }
}
