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
/// Advises the system about the expected use of a mapped address range.
///
/// Nanvix keeps every mapped page resident and never reads ahead, pages out, or reclaims pages, so
/// there is no paging behavior for advice to tune. This function therefore only validates that the
/// requested range is currently mapped.
///
/// # Parameters
///
/// - `addr`: Page-aligned base address of the range to advise on.
/// - `length`: Page-rounded length of the range to advise on.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
pub fn madvise(addr: VirtualAddress, length: usize) -> Result<(), Error> {
    validate_mapped_lock_range("madvise", addr, length)
}
