// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::ptr;
use ::sys::{
    error::Error,
    kcall::pm::{
        __kcall_get_thread_data_area,
        __kcall_set_thread_data_area,
    },
};

//==================================================================================================
// Globals
//==================================================================================================

static mut TEMP_TDA: [u8; 128] = [0; 128];

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Verifies that the thread data area can be updated and restored.
pub fn run() -> Result<(), Error> {
    test_set_and_restore_tda()?;
    Ok(())
}

fn test_set_and_restore_tda() -> Result<(), Error> {
    let original_ptr = __kcall_get_thread_data_area()?;

    let new_ptr: *mut u8 = ptr::addr_of_mut!(TEMP_TDA).cast::<u8>();
    __kcall_set_thread_data_area(new_ptr)?;

    let active_ptr = __kcall_get_thread_data_area()?;
    assert_eq!(active_ptr, new_ptr, "TDA pointer mismatch");

    __kcall_set_thread_data_area(original_ptr)?;
    let restored = __kcall_get_thread_data_area()?;
    assert_eq!(restored, original_ptr, "original TDA pointer was not restored");
    Ok(())
}
