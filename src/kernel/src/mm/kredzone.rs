// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::mem;
use ::sys::error::{
    Error,
    ErrorCode,
};
extern "C" {
    static mut kredzone: usize;
}

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Size of the kernel red zone (in bytes).
///
/// # Note
///
/// - This size should match what is defined in start.S
const KREDZONE_SIZE: usize = 128;

//==================================================================================================
// Standalone Public Functions
//==================================================================================================

///
/// # Description
///
/// Stores a value in the kernel red zone.
///
/// # Parameters
///
/// - `index`: Index in the kernel red zone.
/// - `value`: Value to store.
///
/// # Returns
///
/// Upon success, empty is returned. Upon failure, an error is returned instead.
///
pub fn store(index: usize, value: usize) -> Result<(), Error> {
    // Check if the index is out of bounds.
    if index >= KREDZONE_SIZE / mem::size_of::<usize>() {
        let reason: &str = "index out of bounds";
        error!("store(): index={:?}, value={:?}, (error={})", index, value, reason);
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    // Store the value in the kernel red zone.
    // Safety: the kernel red zone is a global static variable and index is valid.
    unsafe {
        let ptr: *mut usize = core::ptr::addr_of_mut!(kredzone);
        let ptr: *mut usize = ptr.add(index);
        ptr.write_volatile(value);
    }

    Ok(())
}

///
/// # Description
///
/// Loads a value from the kernel red zone.
///
/// # Parameters
///
/// - `index`: Index in the kernel red zone.
///
/// # Returns
///
/// Upon success, the value is returned. Upon failure, an error is returned instead.
///
pub fn load(index: usize) -> Result<usize, Error> {
    // Check if the index is out of bounds.
    if index >= KREDZONE_SIZE / mem::size_of::<usize>() {
        let reason: &str = "index out of bounds";
        error!("load(): index={:?}, (error={})", index, reason);
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    // Load the value from the kernel red zone.
    // Safety: the kernel red zone is a global static variable and index is valid.
    unsafe {
        let ptr: *const usize = core::ptr::addr_of!(kredzone);
        let ptr: *const usize = ptr.add(index);
        Ok(ptr.read_volatile())
    }
}
