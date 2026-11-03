// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::block_header::BlockHeader;
use ::sysapi::{
    ffi::c_void,
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the number of usable bytes in the allocation pointed to by `ptr`.
///
/// This function returns the size (in bytes) of the memory block that was originally requested
/// when the allocation function (e.g., `malloc`, `calloc`, `realloc`, `aligned_alloc`, or
/// `posix_memalign`) returned the pointer `ptr`. The returned size is the logical usable size and
/// may be less than or equal to the actual size reserved internally by the allocator.
///
/// If `ptr` is a null pointer, this function returns `0`.
///
/// # Parameters
///
/// - `ptr`: Pointer to a memory block previously allocated by one of the allocation functions.
///
/// # Returns
///
/// On success, this function returns the usable size of the allocation in bytes. If `ptr` is a
/// null pointer, it returns `0`.
///
/// # Safety
///
/// This function is unsafe because it dereferences internal allocator metadata. The caller must
/// ensure that `ptr` either is null or was returned by a successful allocation from this library
/// and has not yet been freed. Passing any other pointer results in undefined behavior.
///
/// # Notes
///
/// This is a non-standard (GNU extension–like) function provided for diagnostic and optimization
/// purposes. Applications should not rely on the returned size for writing beyond the originally
/// requested bounds.
///
/// # References
///
/// https://www.man7.org/linux/man-pages/man3/malloc_usable_size.3.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn malloc_usable_size(ptr: *mut c_void) -> c_size_t {
    // Check for null pointer.
    if ptr.is_null() {
        return 0;
    }

    let size: usize = BlockHeader::usable_size(ptr.cast::<u8>());

    // `c_size_t` may be narrower than `usize` (it is on the 64-bit host used for unit testing), so
    // saturate on the practically unreachable overflow rather than truncating. On 32-bit targets the
    // value always fits and the conversion is exact.
    size.try_into().unwrap_or(c_size_t::MAX)
}

#[cfg(all(test, feature = "std"))]
mod tests;
