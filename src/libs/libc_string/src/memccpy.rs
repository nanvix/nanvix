// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::mem::align_of;
use ::sysapi::{
    ffi::{
        c_int,
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
/// Copies bytes from one memory region to another, stopping after the first occurrence of byte `c`
/// or after `n` bytes have been copied, whichever comes first.
///
/// # Parameters
///
/// - `dest`: Destination pointer where bytes will be written.
/// - `src`: Source pointer from which bytes will be read.
/// - `c`: Byte value to stop copying after (interpreted as `unsigned char`).
/// - `n`: Maximum number of bytes to copy.
///
/// # Return Value
///
/// Returns a pointer to the byte in `dest` immediately after the copy of `c`, if `c` was found
/// in the first `n` bytes of `src`. Returns a null pointer if `c` was not found.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It performs unchecked writes to the memory region pointed to by `dest`.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn memccpy(
    dest: *mut c_void,
    src: *const c_void,
    c: c_int,
    n: c_size_t,
) -> *mut c_void {
    debug_assert!(!dest.is_null(), "memccpy(): null destination pointer");
    debug_assert!(!src.is_null(), "memccpy(): null source pointer");
    debug_assert!(
        (dest as usize).is_multiple_of(align_of::<c_uchar>()),
        "memccpy(): destination pointer is not properly aligned"
    );
    debug_assert!(
        (src as usize).is_multiple_of(align_of::<c_uchar>()),
        "memccpy(): source pointer is not properly aligned"
    );

    let target: c_uchar = c.to_le_bytes()[0];
    let d: *mut c_uchar = dest.cast::<c_uchar>();
    let s: *const c_uchar = src.cast::<c_uchar>();

    let mut i: c_size_t = 0;
    while i < n {
        let byte: c_uchar = *s.add(i as usize);
        *d.add(i as usize) = byte;
        if byte == target {
            return d.add(i as usize + 1).cast::<c_void>();
        }
        i += 1;
    }

    core::ptr::null_mut()
}

#[cfg(all(test, feature = "std"))]
mod test;
