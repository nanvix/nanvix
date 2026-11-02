// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::cast_sign_loss)]

//==================================================================================================
// Imports
//==================================================================================================

use ::core::mem::align_of;
use ::sysapi::{
    ffi::{
        c_char,
        c_void,
    },
    sys_types::c_size_t,
};

//==================================================================================================
// External Functions
//==================================================================================================

extern "C" {
    fn malloc(size: c_size_t) -> *mut c_void;
    fn strlen(s: *const c_char) -> c_size_t;
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Duplicates at most `n` bytes of a string.
///
/// This function is similar to `strdup`, but copies at most `n` bytes. If `s` is longer than `n`,
/// only `n` bytes are copied, and a terminating null byte is added. The memory is obtained with
/// `malloc` and can be freed with `free`.
///
/// # Parameters
///
/// - `s`: Pointer to the null-terminated string to duplicate.
/// - `n`: Maximum number of bytes to copy.
///
/// # Return Value
///
/// Returns a pointer to the duplicated string, or a null pointer if allocation fails.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It allocates memory with `malloc`.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strndup(s: *const c_char, n: c_size_t) -> *mut c_char {
    debug_assert!(!s.is_null(), "strndup(): null pointer");
    debug_assert!(
        (s as usize).is_multiple_of(align_of::<c_char>()),
        "strndup(): pointer is not properly aligned"
    );

    let len: c_size_t = strlen(s);
    let copy_len: c_size_t = if n < len { n } else { len };
    let ptr: *mut c_void = malloc(copy_len + 1);
    if ptr.is_null() {
        return core::ptr::null_mut();
    }

    let dest: *mut c_char = ptr.cast::<c_char>();
    let mut i: usize = 0;
    let total: usize = copy_len as usize;
    while i < total {
        *dest.add(i) = *s.add(i);
        i += 1;
    }
    *dest.add(total) = 0 as c_char;

    dest
}

#[cfg(all(test, feature = "std"))]
mod test;
