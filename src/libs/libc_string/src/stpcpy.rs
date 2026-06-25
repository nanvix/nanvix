// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
/// Copies a string and returns a pointer to its end.
///
/// This function copies the null-terminated string pointed to by `src` (including the terminating
/// null byte) into the buffer pointed to by `dest`, exactly like `strcpy()`, but returns a pointer
/// to the terminating null byte written in `dest` rather than to its start. It is a GNU extension
/// that is convenient for chaining successive copies.
///
/// # Parameters
///
/// - `dest`: Destination buffer where the string will be written.
/// - `src`: Pointer to the null-terminated source string.
///
/// # Return Value
///
/// Returns a pointer to the terminating null byte written in `dest`.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It performs unchecked writes to the memory region pointed to by `dest`.
///
/// It is safe to call this function if and only if the following conditions are met:
/// - `src` points to a valid, null-terminated string.
/// - `dest` points to a writable memory region large enough to hold the string in `src`
///   (including its terminating null byte).
/// - The source and destination regions do not overlap.
///
/// Violating any of these conditions results in undefined behavior.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn stpcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char {
    debug_assert!(!dest.is_null(), "stpcpy(): null destination pointer");
    debug_assert!(!src.is_null(), "stpcpy(): null source pointer");
    debug_assert!(
        (dest as usize).is_multiple_of(align_of::<c_char>()),
        "stpcpy(): destination pointer is not properly aligned"
    );
    debug_assert!(
        (src as usize).is_multiple_of(align_of::<c_char>()),
        "stpcpy(): source pointer is not properly aligned"
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
    dest.add(i)
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::stpcpy;
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
    fn test_stpcpy_copies_string() {
        let src: Vec<c_char> = make_c_string(b"hello");
        let mut dest: Vec<c_char> = vec![0 as c_char; src.len()];
        unsafe { stpcpy(dest.as_mut_ptr(), src.as_ptr()) };
        assert_eq!(dest, src, "stpcpy should copy the whole string including the terminator");
    }

    #[test]
    fn test_stpcpy_returns_end_pointer() {
        let src: Vec<c_char> = make_c_string(b"hello");
        let mut dest: Vec<c_char> = vec![0 as c_char; src.len()];
        let ret: *mut c_char = unsafe { stpcpy(dest.as_mut_ptr(), src.as_ptr()) };
        let offset: usize = unsafe { ret.offset_from(dest.as_ptr()) } as usize;
        assert_eq!(offset, 5, "stpcpy should return a pointer to the terminating null byte");
        assert_eq!(unsafe { *ret }, 0 as c_char, "the returned pointer must point at a null byte");
    }

    #[test]
    fn test_stpcpy_empty_string() {
        let src: Vec<c_char> = make_c_string(b"");
        let mut dest: Vec<c_char> = vec![0x7F as c_char; 1];
        let ret: *mut c_char = unsafe { stpcpy(dest.as_mut_ptr(), src.as_ptr()) };
        let offset: usize = unsafe { ret.offset_from(dest.as_ptr()) } as usize;
        assert_eq!(offset, 0, "stpcpy of an empty string should return the destination start");
        assert_eq!(dest[0], 0 as c_char, "stpcpy should write the terminating null byte");
    }
}
