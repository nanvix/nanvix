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
/// Copies at most `n` bytes from a null-terminated string.
///
/// This function copies at most `n` bytes from the string pointed to by `src` to the buffer
/// pointed to by `dest`. If the length of `src` is less than `n`, the remainder of `dest` is
/// padded with null bytes. If the length of `src` is greater than or equal to `n`, `dest` is NOT
/// null-terminated.
///
/// # Parameters
///
/// - `dest`: Pointer to the destination buffer.
/// - `src`: Pointer to the source null-terminated string.
/// - `n`: Maximum number of bytes to copy.
///
/// # Return Value
///
/// Returns the original destination pointer `dest`.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It writes to the memory region pointed to by `dest` without bounds checking.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strncpy(
    dest: *mut c_char,
    src: *const c_char,
    n: c_size_t,
) -> *mut c_char {
    debug_assert!(!dest.is_null(), "strncpy(): null destination pointer");
    debug_assert!(!src.is_null(), "strncpy(): null source pointer");
    debug_assert!(
        (dest as usize).is_multiple_of(align_of::<c_char>()),
        "strncpy(): destination pointer is not properly aligned"
    );
    debug_assert!(
        (src as usize).is_multiple_of(align_of::<c_char>()),
        "strncpy(): source pointer is not properly aligned"
    );

    let n: usize = n as usize;
    let mut i: usize = 0;

    // Copy bytes from src until null terminator or n bytes.
    while i < n {
        let c: c_char = *src.add(i);
        *dest.add(i) = c;
        if c == 0 {
            i += 1;
            break;
        }
        i += 1;
    }

    // Pad remainder with null bytes.
    while i < n {
        *dest.add(i) = 0 as c_char;
        i += 1;
    }

    dest
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::strncpy;
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

    #[test]
    fn test_strncpy_exact_fit() {
        let src: Vec<c_char> = make_c_string(b"abc");
        let mut dest: Vec<c_char> = vec![0x7F as c_char; 4];
        unsafe { strncpy(dest.as_mut_ptr(), src.as_ptr(), 4) };
        assert_eq!(dest[0], c_char::try_from(b'a').expect("ASCII fits in c_char"));
        assert_eq!(dest[1], c_char::try_from(b'b').expect("ASCII fits in c_char"));
        assert_eq!(dest[2], c_char::try_from(b'c').expect("ASCII fits in c_char"));
        assert_eq!(dest[3], 0 as c_char);
    }

    #[test]
    fn test_strncpy_short_source() {
        let src: Vec<c_char> = make_c_string(b"ab");
        let mut dest: Vec<c_char> = vec![0x7F as c_char; 6];
        unsafe { strncpy(dest.as_mut_ptr(), src.as_ptr(), 6) };
        assert_eq!(dest[0], c_char::try_from(b'a').expect("ASCII fits in c_char"));
        assert_eq!(dest[1], c_char::try_from(b'b').expect("ASCII fits in c_char"));
        // Remaining bytes should be null-padded.
        for (j, &c) in dest[2..6].iter().enumerate() {
            let idx: usize = j + 2;
            assert_eq!(c, 0 as c_char, "byte at index {idx} should be null");
        }
    }

    #[test]
    fn test_strncpy_long_source() {
        let src: Vec<c_char> = make_c_string(b"abcdef");
        let mut dest: Vec<c_char> = vec![0x7F as c_char; 3];
        unsafe { strncpy(dest.as_mut_ptr(), src.as_ptr(), 3) };
        assert_eq!(dest[0], c_char::try_from(b'a').expect("ASCII fits in c_char"));
        assert_eq!(dest[1], c_char::try_from(b'b').expect("ASCII fits in c_char"));
        assert_eq!(dest[2], c_char::try_from(b'c').expect("ASCII fits in c_char"));
        // Not null-terminated when source is longer than n.
    }
}
