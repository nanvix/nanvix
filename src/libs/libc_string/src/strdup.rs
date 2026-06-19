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
mod test {
    use super::strdup;
    use ::std::vec::Vec;
    use ::sysapi::ffi::c_char;

    fn make_c_string(bytes: &[u8]) -> Vec<c_char> {
        let mut v: Vec<c_char> = bytes
            .iter()
            .map(|b| c_char::try_from(*b).expect("byte fits in c_char"))
            .collect();
        v.push(0 as c_char);
        v
    }

    fn c_str_to_bytes(p: *const c_char) -> Vec<u8> {
        let mut v: Vec<u8> = Vec::new();
        let mut i: usize = 0;
        unsafe {
            while *p.add(i) != 0 {
                v.push(u8::from_ne_bytes((*p.add(i)).to_ne_bytes()));
                i += 1;
            }
        }
        v
    }

    extern "C" {
        fn free(ptr: *mut core::ffi::c_void);
    }

    #[test]
    fn test_strdup_basic() {
        let src: Vec<c_char> = make_c_string(b"hello");
        let dup: *mut c_char = unsafe { strdup(src.as_ptr()) };
        assert!(!dup.is_null(), "strdup should not return null");
        assert_eq!(c_str_to_bytes(dup), b"hello");
        // Ensure it is a different allocation.
        assert_ne!(dup as *const c_char, src.as_ptr());
        unsafe { free(dup.cast()) };
    }

    #[test]
    fn test_strdup_empty() {
        let src: Vec<c_char> = make_c_string(b"");
        let dup: *mut c_char = unsafe { strdup(src.as_ptr()) };
        assert!(!dup.is_null(), "strdup should not return null for empty string");
        assert_eq!(c_str_to_bytes(dup), b"");
        unsafe { free(dup.cast()) };
    }
}
