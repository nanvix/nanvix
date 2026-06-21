// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys::mman::syscalls::validate_mapped_lock_range;
use ::sys::{
    error::Error,
    mm::VirtualAddress,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Locks a mapped address range into memory.
///
/// Nanvix does not swap pages, so mapped pages are already resident. This function only validates
/// that the requested range is currently mapped.
///
/// # Parameters
///
/// - `addr`: Page-aligned base address of the range to lock.
/// - `length`: Page-rounded length of the range to lock.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
pub fn mlock(addr: VirtualAddress, length: usize) -> Result<(), Error> {
    validate_mapped_lock_range("mlock", addr, length)
}
