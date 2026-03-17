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
    loop {
        core::hint::spin_loop();
    }
}
