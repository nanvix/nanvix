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
/// Unlocks a mapped address range.
///
/// Nanvix does not swap pages, so unlocking mapped pages has no residency effect. This function
/// only validates that the requested range is currently mapped.
///
/// # Parameters
///
/// - `addr`: Page-aligned base address of the range to unlock.
/// - `length`: Page-rounded length of the range to unlock.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
pub fn munlock(addr: VirtualAddress, length: usize) -> Result<(), Error> {
    validate_mapped_lock_range("munlock", addr, length)
}
