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
use ::sysapi::ffi::c_char;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Copies a null-terminated string.
///
/// This function copies the string pointed to by `src`, including the terminating null byte, to the
/// buffer pointed to by `dest`. The strings must not overlap, and the destination buffer must be
/// large enough to receive the copy.
///
/// # Parameters
///
/// - `dest`: Pointer to the destination buffer.
/// - `src`: Pointer to the source null-terminated string.
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
pub unsafe extern "C" fn strcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char {
    debug_assert!(!dest.is_null(), "strcpy(): null destination pointer");
    debug_assert!(!src.is_null(), "strcpy(): null source pointer");
    debug_assert!(
        (dest as usize).is_multiple_of(align_of::<c_char>()),
        "strcpy(): destination pointer is not properly aligned"
    );
    debug_assert!(
        (src as usize).is_multiple_of(align_of::<c_char>()),
        "strcpy(): source pointer is not properly aligned"
    );

    let mut i: usize = 0;
    loop {
        let c: c_char = *src.add(i);
        *dest.add(i) = c;
        if c == 0 {
            break;
        }
        i += 1;
    }
    dest
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::strcpy;
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
    fn test_strcpy_basic() {
        let src: Vec<c_char> = make_c_string(b"hello");
        let mut dest: Vec<c_char> = vec![0 as c_char; 10];
        let ret: *mut c_char = unsafe { strcpy(dest.as_mut_ptr(), src.as_ptr()) };
        assert_eq!(ret, dest.as_mut_ptr(), "strcpy should return dest");
        assert_eq!(dest[0], c_char::try_from(b'h').expect("ASCII fits in c_char"));
        assert_eq!(dest[5], 0 as c_char);
    }

    #[test]
    fn test_strcpy_empty() {
        let src: Vec<c_char> = make_c_string(b"");
        let mut dest: Vec<c_char> = vec![0x7F as c_char; 5];
        unsafe { strcpy(dest.as_mut_ptr(), src.as_ptr()) };
        assert_eq!(dest[0], 0 as c_char, "strcpy of empty string should write null terminator");
    }
}
