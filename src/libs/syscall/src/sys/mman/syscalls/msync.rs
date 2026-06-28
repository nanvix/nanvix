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
/// Synchronizes a mapped address range with its backing store.
///
/// Nanvix backs every mapping with anonymous physical memory and never caches mapped pages in a
/// separate store, so the in-memory contents are always the authoritative copy. Synchronizing a
/// range therefore has nothing to write back, and this function only validates that the requested
/// range is currently mapped.
///
/// # Parameters
///
/// - `addr`: Page-aligned base address of the range to synchronize.
/// - `length`: Page-rounded length of the range to synchronize.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
pub fn msync(addr: VirtualAddress, length: usize) -> Result<(), Error> {
    validate_mapped_lock_range("msync", addr, length)
}
