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
mod test {
    use super::strndup;
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
    fn test_strndup_n_less_than_length() {
        let src: Vec<c_char> = make_c_string(b"hello world");
        let dup: *mut c_char = unsafe { strndup(src.as_ptr(), 5) };
        assert!(!dup.is_null(), "strndup should not return null");
        assert_eq!(c_str_to_bytes(dup), b"hello");
        unsafe { free(dup.cast()) };
    }

    #[test]
    fn test_strndup_n_greater_than_length() {
        let src: Vec<c_char> = make_c_string(b"hi");
        let dup: *mut c_char = unsafe { strndup(src.as_ptr(), 100) };
        assert!(!dup.is_null(), "strndup should not return null");
        assert_eq!(c_str_to_bytes(dup), b"hi");
        unsafe { free(dup.cast()) };
    }
}
