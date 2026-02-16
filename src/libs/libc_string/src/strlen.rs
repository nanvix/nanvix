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
/// Calculates the length of a string.
///
/// This function computes the length of the string pointed to by `s`, excluding the terminating
/// null byte (`'\0'`).
///
/// # Parameters
///
/// - `s`: Pointer to the null-terminated string whose length is to be calculated.
///
/// # Return Value
///
/// This function returns the number of characters in the string pointed to by `s`, excluding the
/// terminating null byte. If `s` is a null pointer, the behavior is undefined.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It reads from the memory region pointed to by `s` without bounds checking.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strlen(s: *const c_char) -> c_size_t {
    debug_assert!(!s.is_null(), "strlen(): null pointer");
    debug_assert!((s as usize) < isize::MAX as usize, "strlen(): pointer too large");
    debug_assert!(
        (s as usize).is_multiple_of(align_of::<c_char>()),
        "strlen(): pointer is not properly aligned"
    );

    let mut i: c_size_t = 0;
    while *s.add(i as usize) != 0 {
        i += 1;
    }
    i
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::strlen;
    use ::std::vec::Vec;
    use ::sysapi::ffi::c_char;

    // Helper to build a null-terminated Vec<c_char> from bytes (without the final null).
    fn make_c_string(bytes: &[u8]) -> Vec<c_char> {
        let mut v: Vec<c_char> = bytes
            .iter()
            .map(|b| c_char::try_from(*b).expect("byte fits in c_char"))
            .collect();
        v.push(0 as c_char); // null terminator
        v
    }

    #[test]
    fn test_strlen_empty_string() {
        let buf: Vec<c_char> = make_c_string(b"");
        let len: usize = unsafe { strlen(buf.as_ptr()) as usize };
        assert_eq!(len, 0, "Length of empty string should be 0");
    }

    #[test]
    fn test_strlen_basic_string() {
        let buf: Vec<c_char> = make_c_string(b"hello");
        let len: usize = unsafe { strlen(buf.as_ptr()) as usize };
        assert_eq!(len, 5, "Length of 'hello' should be 5");
    }

    #[test]
    fn test_strlen_long_string() {
        // Use a moderately long string to exercise loop.
        let data: Vec<u8> = (0..100u8).map(|i| b'a' + (i % 26)).collect();
        let buf: Vec<c_char> = make_c_string(&data);
        let len: usize = unsafe { strlen(buf.as_ptr()) as usize };
        assert_eq!(len, data.len(), "Length mismatch for long string");
    }

    #[test]
    fn test_strlen_embedded_null() {
        // Build bytes with an early null followed by other bytes; strlen should stop at first null.
        let mut raw: Vec<u8> = b"abc".to_vec();
        raw.push(0); // embedded null
        raw.extend_from_slice(b"def");
        let mut buf: Vec<c_char> = raw
            .iter()
            .map(|b| c_char::try_from(*b).expect("byte fits in c_char"))
            .collect();
        buf.push(0 as c_char); // final terminator (technically redundant after embedded null but ok)
        let len: usize = unsafe { strlen(buf.as_ptr()) as usize };
        assert_eq!(len, 3, "strlen should stop at first embedded null");
    }

    #[test]
    fn test_strlen_unaligned_pointer() {
        // Allocate with a leading padding byte to create an intentionally unaligned start.
        let storage: Vec<c_char> = make_c_string(b"nanvix");
        // Prepend one dummy byte to shift alignment.
        let mut padded: Vec<c_char> = Vec::with_capacity(storage.len() + 1);
        padded.push(c_char::try_from(b'X').expect("ASCII fits in c_char")); // padding (not a null)
        padded.extend_from_slice(&storage);
        let unaligned_ptr: *const c_char = unsafe { padded.as_ptr().add(1) }; // Points to start of "nanvix" string.
        let len: usize = unsafe { strlen(unaligned_ptr) as usize };
        assert_eq!(len, 6, "Length of 'nanvix' should be 6 even from unaligned pointer");
    }

    #[test]
    fn test_strlen_non_ascii_bytes() {
        // Bytes above 0x7F are still single bytes, strlen counts until null.
        let bytes: [u8; 4] = [0xFF, 0x80, 0xC3, 0x00];
        let mut buf: Vec<c_char> = bytes[..3].iter().map(|b| i8::from_ne_bytes([*b])).collect();
        buf.push(0 as c_char);
        let len: usize = unsafe { strlen(buf.as_ptr()) as usize };
        assert_eq!(len, 3, "strlen should count raw bytes irrespective of UTF-8 validity");
    }
}
