// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::{
        c_int,
        c_void,
    },
    sys_types::c_size_t,
};

unsafe extern "C" {
    pub fn malloc(size: c_size_t) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Allocates memory with a specified alignment.
///
/// # Parameters
///
/// - `memptr`: Pointer to a pointer where the allocated memory address will be stored.
/// - `alignment`: The alignment requirement for the allocated memory.
/// - `size`: The size of the memory block to be allocated.
///
/// # Returns
///
/// On success, returns `0`. On failure, returns an error code.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if all the following conditions are met:
///
/// - `memptr` points to a valid memory location that can store a `*mut c_void`.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn posix_memalign(
    memptr: *mut *mut c_void,
    alignment: c_size_t,
    size: c_size_t,
) -> c_int {
    ::syslog::trace!("posix_memalign(): alignment={alignment:?}, size={size:?}");

    // TODO: Implement proper aligned memory allocation instead of using malloc.
    // See: https://github.com/nanvix/nanvix/issues/648.

    // Check if memptr is null.
    if memptr.is_null() {
        ::syslog::error!(
            "posix_memalign(): invalid storage location (alignment={alignment:?}, size={size:?})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if size is invalid.
    if size == 0 {
        ::syslog::error!(
            "posix_memalign(): invalid allocation size (alignment={alignment:?}, size={size:?})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Check if alignment is a power of two.
    if alignment == 0 || (alignment & (alignment - 1)) != 0 {
        ::syslog::error!(
            "posix_memalign(): invalid alignment (alignment={alignment:?}, size={size:?})"
        );
        return ErrorCode::InvalidArgument.get();
    }

    // Allocate memory.
    let ptr: *mut c_void = malloc(size);

    // Check if allocation was successful.
    if ptr.is_null() {
        ::syslog::error!(
            "posix_memalign(): memory allocation failed (alignment={alignment:?}, size={size:?})"
        );
        return ErrorCode::OutOfMemory.get();
    }

    // Check if the allocated memory address is aligned.
    if (ptr as usize) & (alignment as usize - 1) != 0 {
        free(ptr);
        ::syslog::warn!(
            "posix_memalign(): failed to allocate aligned memory area (alignment={alignment:?}, \
             size={size:?})"
        );
        return ErrorCode::OutOfMemory.get();
    }

    // Set the allocated memory address.
    *memptr = ptr;

    0
}
