// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::common::{
    StressError,
    raw_pointer_address,
};
use ::sys::kcall::pm::{
    __kcall_get_thread_data_area,
    __kcall_set_thread_data_area,
};

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Saves and restores the thread data area while swapping in a scratch buffer, mirroring runtimes
/// that temporarily repurpose TDA for TLS or per-thread metadata during context switches.
///
/// # Returns
///
/// `Ok(())` on success or an error if thread data area operations fail.
///
pub fn run() -> Result<(), StressError> {
    let saved_tda: *mut u8 = __kcall_get_thread_data_area()?;

    let inner_result: Result<(), StressError> = (|| {
        let mut scratch: [u8; 64] = [0; 64];
        scratch.fill(0xbc);
        let scratch_ptr: *mut u8 = scratch.as_mut_ptr();
        __kcall_set_thread_data_area(scratch_ptr)?;

        let observed: *mut u8 = __kcall_get_thread_data_area()?;
        let scratch_addr: usize = raw_pointer_address(scratch_ptr);
        let observed_addr: usize = raw_pointer_address(observed);

        assert_eq!(observed_addr, scratch_addr, "thread data area mismatch");

        Ok(())
    })();

    let restore_result: Result<(), StressError> = __kcall_set_thread_data_area(saved_tda);
    inner_result?;
    restore_result
}
