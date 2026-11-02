// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::cast_sign_loss)]

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    ffi::{
        c_uchar,
        c_void,
    },
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Copies bytes from one memory region to another and returns a pointer past the end.
///
/// This function copies exactly `len` bytes from the memory region pointed to by `src` to the
/// memory region pointed to by `dest`, exactly like `memcpy()`, but returns a pointer to the byte
/// following the last written byte (`dest + len`) rather than `dest`. It is a GNU extension that is
/// convenient for chaining successive copies.
///
/// Behavior is undefined if the source and destination memory regions overlap.
///
/// # Parameters
///
/// - `dest`: Destination pointer where bytes will be written.
/// - `src`: Source pointer from which bytes will be read.
/// - `len`: Number of bytes to copy.
///
/// # Return Value
///
/// This function returns `dest + len` (a pointer to the byte after the last one written).
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It performs unchecked writes to the memory region pointed to by `dest`.
///
/// It is safe to call this function if and only if the following conditions are met:
/// - `dest` points to a valid and writable memory region of at least `len` bytes.
/// - `src` points to a valid and readable memory region of at least `len` bytes.
/// - `len` does not exceed `isize::MAX`.
/// - `dest` and `src` do not point to overlapping memory regions.
///
/// Violating any of these conditions results in undefined behavior.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn mempcpy(
    dest: *mut c_void,
    src: *const c_void,
    len: c_size_t,
) -> *mut c_void {
    debug_assert!(!dest.is_null(), "mempcpy(): null destination pointer");
    debug_assert!(!src.is_null(), "mempcpy(): null source pointer");

    crate::memcpy::memcpy(dest, src, len);
    dest.cast::<c_uchar>().add(len as usize).cast::<c_void>()
}

#[cfg(all(test, feature = "std"))]
mod test;
