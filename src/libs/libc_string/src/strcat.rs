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
/// Appends a null-terminated string to another.
///
/// This function appends the string pointed to by `src` to the end of the string pointed to by
/// `dest`, overwriting the null terminator at the end of `dest`, and then adding a terminating null
/// byte. The strings must not overlap, and the `dest` buffer must be large enough.
///
/// # Parameters
///
/// - `dest`: Pointer to the destination null-terminated string.
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
pub unsafe extern "C" fn strcat(dest: *mut c_char, src: *const c_char) -> *mut c_char {
    debug_assert!(!dest.is_null(), "strcat(): null destination pointer");
    debug_assert!(!src.is_null(), "strcat(): null source pointer");
    debug_assert!(
        (dest as usize).is_multiple_of(align_of::<c_char>()),
        "strcat(): destination pointer is not properly aligned"
    );
    debug_assert!(
        (src as usize).is_multiple_of(align_of::<c_char>()),
        "strcat(): source pointer is not properly aligned"
    );

    // Find the end of dest.
    let mut d: usize = 0;
    while *dest.add(d) != 0 {
        d += 1;
    }

    // Copy src to end of dest.
    let mut s: usize = 0;
    loop {
        let c: c_char = *src.add(s);
        *dest.add(d) = c;
        if c == 0 {
            break;
        }
        d += 1;
        s += 1;
    }

    dest
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::strcat;
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
    fn test_strcat_basic() {
        let mut dest: Vec<c_char> = vec![0 as c_char; 20];
        let hello: Vec<c_char> = make_c_string(b"hello");
        let world: Vec<c_char> = make_c_string(b" world");
        // Copy "hello" into dest first.
        for (i, &c) in hello.iter().enumerate() {
            dest[i] = c;
        }
        unsafe { strcat(dest.as_mut_ptr(), world.as_ptr()) };
        let expected: Vec<c_char> = make_c_string(b"hello world");
        for (i, &c) in expected.iter().enumerate() {
            assert_eq!(dest[i], c, "mismatch at index {i}");
        }
    }

    #[test]
    fn test_strcat_empty_src() {
        let mut dest: Vec<c_char> = vec![0 as c_char; 10];
        let hello: Vec<c_char> = make_c_string(b"hello");
        for (i, &c) in hello.iter().enumerate() {
            dest[i] = c;
        }
        let empty: Vec<c_char> = make_c_string(b"");
        unsafe { strcat(dest.as_mut_ptr(), empty.as_ptr()) };
        assert_eq!(dest[0], c_char::try_from(b'h').expect("ASCII fits in c_char"));
        assert_eq!(dest[5], 0 as c_char);
    }

    #[test]
    fn test_strcat_empty_dest() {
        let mut dest: Vec<c_char> = vec![0 as c_char; 10];
        let src: Vec<c_char> = make_c_string(b"abc");
        unsafe { strcat(dest.as_mut_ptr(), src.as_ptr()) };
        assert_eq!(dest[0], c_char::try_from(b'a').expect("ASCII fits in c_char"));
        assert_eq!(dest[1], c_char::try_from(b'b').expect("ASCII fits in c_char"));
        assert_eq!(dest[2], c_char::try_from(b'c').expect("ASCII fits in c_char"));
        assert_eq!(dest[3], 0 as c_char);
    }
}
