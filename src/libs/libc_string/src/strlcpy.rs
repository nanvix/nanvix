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
    ffi::c_char,
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Size-bounded string copying.
///
/// This function copies up to `size - 1` characters from the string `src` to `dest`,
/// null-terminating the result if `size` is not zero. Unlike `strncpy`, `strlcpy` always
/// null-terminates the result (as long as `size > 0`).
///
/// # Parameters
///
/// - `dest`: Pointer to the destination buffer.
/// - `src`: Pointer to the source null-terminated string.
/// - `size`: Size of the destination buffer.
///
/// # Return Value
///
/// Returns the total length of the string that would have been created if there was unlimited
/// space (i.e., `strlen(src)`). If the return value is >= `size`, truncation occurred.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It writes to the memory region pointed to by `dest` without bounds checking.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strlcpy(
    dest: *mut c_char,
    src: *const c_char,
    size: c_size_t,
) -> c_size_t {
    debug_assert!(!dest.is_null(), "strlcpy(): null destination pointer");
    debug_assert!(!src.is_null(), "strlcpy(): null source pointer");
    debug_assert!(
        (dest as usize).is_multiple_of(align_of::<c_char>()),
        "strlcpy(): destination pointer is not properly aligned"
    );
    debug_assert!(
        (src as usize).is_multiple_of(align_of::<c_char>()),
        "strlcpy(): source pointer is not properly aligned"
    );

    let size: usize = size as usize;

    // Calculate strlen(src) using c_size_t to avoid usize->c_size_t cast.
    let mut src_len: c_size_t = 0;
    while *src.add(src_len as usize) != 0 {
        src_len += 1;
    }

    if size > 0 {
        let copy_len: usize = if (src_len as usize) < size {
            src_len as usize
        } else {
            size - 1
        };
        let mut i: usize = 0;
        while i < copy_len {
            *dest.add(i) = *src.add(i);
            i += 1;
        }
        *dest.add(copy_len) = 0 as c_char;
    }

    src_len
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::strlcpy;
    use ::std::vec::Vec;
    use ::sysapi::{
        ffi::c_char,
        sys_types::c_size_t,
    };

    fn make_c_string(bytes: &[u8]) -> Vec<c_char> {
        let mut v: Vec<c_char> = bytes
            .iter()
            .map(|b| c_char::try_from(*b).expect("byte fits in c_char"))
            .collect();
        v.push(0 as c_char);
        v
    }

    #[test]
    fn test_strlcpy_normal() {
        let src: Vec<c_char> = make_c_string(b"hello");
        let mut dest: Vec<c_char> = vec![0x7F as c_char; 10];
        let ret: c_size_t = unsafe { strlcpy(dest.as_mut_ptr(), src.as_ptr(), 10) };
        assert_eq!(ret as usize, 5, "strlcpy should return strlen(src)");
        assert_eq!(dest[0], c_char::try_from(b'h').expect("ASCII fits in c_char"));
        assert_eq!(dest[5], 0 as c_char);
    }

    #[test]
    fn test_strlcpy_truncation() {
        let src: Vec<c_char> = make_c_string(b"hello world");
        let mut dest: Vec<c_char> = vec![0x7F as c_char; 6];
        let ret: c_size_t = unsafe { strlcpy(dest.as_mut_ptr(), src.as_ptr(), 6) };
        assert_eq!(ret as usize, 11, "strlcpy should return full strlen(src)");
        assert_eq!(dest[5], 0 as c_char, "dest should be null-terminated");
        assert_eq!(dest[0], c_char::try_from(b'h').expect("ASCII fits in c_char"));
        assert_eq!(dest[4], c_char::try_from(b'o').expect("ASCII fits in c_char"));
    }

    #[test]
    fn test_strlcpy_zero_size() {
        let src: Vec<c_char> = make_c_string(b"hello");
        let mut dest: Vec<c_char> = vec![0x7F as c_char; 5];
        let ret: c_size_t = unsafe { strlcpy(dest.as_mut_ptr(), src.as_ptr(), 0) };
        assert_eq!(ret as usize, 5, "strlcpy should return strlen(src) even with size 0");
        assert_eq!(dest[0], 0x7F as c_char, "dest should be unchanged with size 0");
    }
}
