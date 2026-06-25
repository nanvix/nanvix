// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
/// Copies at most `n` bytes of a string and returns a pointer to its end.
///
/// This function copies at most `n` bytes from the null-terminated string pointed to by `src` into
/// the buffer pointed to by `dest`, exactly like `strncpy()` (padding `dest` with null bytes when
/// `src` is shorter than `n`), but returns a pointer to the first null byte written in `dest`, or
/// to `dest + n` when no null byte was written. It is a POSIX extension convenient for chaining
/// successive copies.
///
/// Note that, as with `strncpy()`, if there is no null byte among the first `n` bytes of `src`, the
/// string written to `dest` is not null-terminated.
///
/// # Parameters
///
/// - `dest`: Destination buffer where the bytes will be written.
/// - `src`: Pointer to the null-terminated source string.
/// - `n`: Maximum number of bytes to write.
///
/// # Return Value
///
/// Returns a pointer to the first null byte written in `dest`, or `dest + n` if no null byte was
/// written.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It performs unchecked writes to the memory region pointed to by `dest`.
///
/// It is safe to call this function if and only if the following conditions are met:
/// - `src` points to a valid, null-terminated string.
/// - `dest` points to a writable memory region of at least `n` bytes.
/// - The source and destination regions do not overlap.
///
/// Violating any of these conditions results in undefined behavior.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn stpncpy(
    dest: *mut c_char,
    src: *const c_char,
    n: c_size_t,
) -> *mut c_char {
    debug_assert!(!dest.is_null(), "stpncpy(): null destination pointer");
    debug_assert!(!src.is_null(), "stpncpy(): null source pointer");
    debug_assert!(
        (dest as usize).is_multiple_of(align_of::<c_char>()),
        "stpncpy(): destination pointer is not properly aligned"
    );
    debug_assert!(
        (src as usize).is_multiple_of(align_of::<c_char>()),
        "stpncpy(): source pointer is not properly aligned"
    );

    let n: usize = n as usize;
    let mut i: usize = 0;

    // Copy bytes from src until null terminator or n bytes.
    while i < n {
        let c: c_char = *src.add(i);
        *dest.add(i) = c;
        if c == 0 {
            break;
        }
        i += 1;
    }

    // The end pointer is the position of the first null byte written, or dest + n when src is at
    // least n bytes long (no null byte was written).
    let end: *mut c_char = dest.add(i);

    // Pad the remainder of the buffer with null bytes.
    while i < n {
        *dest.add(i) = 0 as c_char;
        i += 1;
    }

    end
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::stpncpy;
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
    fn test_stpncpy_copies_and_pads() {
        let src: Vec<c_char> = make_c_string(b"abc");
        let mut dest: Vec<c_char> = vec![0x7F as c_char; 6];
        let ret: *mut c_char = unsafe { stpncpy(dest.as_mut_ptr(), src.as_ptr(), 6) };
        let offset: usize = unsafe { ret.offset_from(dest.as_ptr()) } as usize;
        assert_eq!(offset, 3, "stpncpy should return a pointer to the first null byte");
        let mut expected: Vec<c_char> = b"abc"
            .iter()
            .map(|b| c_char::try_from(*b).expect("byte fits in c_char"))
            .collect();
        expected.resize(6, 0 as c_char);
        assert_eq!(dest, expected, "stpncpy should pad the remainder with null bytes");
    }

    #[test]
    fn test_stpncpy_truncates_without_terminator() {
        let src: Vec<c_char> = make_c_string(b"abcdef");
        let mut dest: Vec<c_char> = vec![0x7F as c_char; 3];
        let ret: *mut c_char = unsafe { stpncpy(dest.as_mut_ptr(), src.as_ptr(), 3) };
        let offset: usize = unsafe { ret.offset_from(dest.as_ptr()) } as usize;
        assert_eq!(offset, 3, "stpncpy should return dest + n when no null byte is written");
        let expected: Vec<c_char> = b"abc"
            .iter()
            .map(|b| c_char::try_from(*b).expect("byte fits in c_char"))
            .collect();
        assert_eq!(dest, expected, "stpncpy should copy exactly n bytes");
    }

    #[test]
    fn test_stpncpy_zero_n() {
        let src: Vec<c_char> = make_c_string(b"abc");
        let mut dest: Vec<c_char> = vec![0x7F as c_char; 1];
        let ret: *mut c_char = unsafe { stpncpy(dest.as_mut_ptr(), src.as_ptr(), 0) };
        let offset: usize = unsafe { ret.offset_from(dest.as_ptr()) } as usize;
        assert_eq!(offset, 0, "stpncpy with n == 0 should return the destination start");
        assert_eq!(dest[0], 0x7F as c_char, "stpncpy with n == 0 must not write anything");
    }
}
