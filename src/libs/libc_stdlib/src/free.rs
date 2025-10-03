// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::block_header::BlockHeader;
use ::sysapi::ffi::c_void;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Deallocates a block of memory that was previously allocated by `aligned_alloc`, `calloc`,
/// `malloc`, `posix_memalign`, or `realloc`.
///
/// # Parameters
///
/// - `ptr`: Pointer to the memory block to be deallocated.
///
/// # Safety
///
/// This function is unsafe because it interacts with the global memory allocator.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/free.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn free(ptr: *mut c_void) {
    // Check for null pointer deallocation.
    if ptr.is_null() {
        return;
    }

    let _ = BlockHeader::free(ptr.cast::<u8>());
}

#[cfg(all(test, feature = "std"))]
mod tests {
    #[test]
    fn null_deallocation() {
        unsafe {
            super::free(core::ptr::null_mut());
        }
    }
}
