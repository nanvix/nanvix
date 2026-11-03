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
/// Duplicates a string.
///
/// This function allocates sufficient memory for a copy of the string `s`, copies the string, and
/// returns a pointer to it. The memory is obtained with `malloc` and can be freed with `free`.
///
/// # Parameters
///
/// - `s`: Pointer to the null-terminated string to duplicate.
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
pub unsafe extern "C" fn strdup(s: *const c_char) -> *mut c_char {
    debug_assert!(!s.is_null(), "strdup(): null pointer");
    debug_assert!(
        (s as usize).is_multiple_of(align_of::<c_char>()),
        "strdup(): pointer is not properly aligned"
    );

    let len: c_size_t = strlen(s);
    let ptr: *mut c_void = malloc(len + 1);
    if ptr.is_null() {
        return core::ptr::null_mut();
    }

    let dest: *mut c_char = ptr.cast::<c_char>();
    let mut i: usize = 0;
    let total: usize = (len + 1) as usize;
    while i < total {
        *dest.add(i) = *s.add(i);
        i += 1;
    }

    dest
}

#[cfg(all(test, feature = "std"))]
mod test;
