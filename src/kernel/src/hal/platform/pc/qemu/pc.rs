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
        ::sys::arch::io::out16(0x604, 0x2000);
    };
    loop {
        core::hint::spin_loop();
    }
}
