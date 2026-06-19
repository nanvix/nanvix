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
// State
//==================================================================================================

static mut LAST: *mut c_char = core::ptr::null_mut();

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Checks whether `c` is one of the delimiter characters in `delim`.
unsafe fn is_delim(c: c_char, delim: *const c_char) -> bool {
    let mut d: usize = 0;
    while *delim.add(d) != 0 {
        if c == *delim.add(d) {
            return true;
        }
        d += 1;
    }
    false
}

///
/// # Description
///
/// Extracts tokens from strings.
///
/// This function breaks a string into a sequence of zero or more non-empty tokens. On the first
/// call, `s` should point to the string to be parsed. On subsequent calls to obtain subsequent
/// tokens, `s` should be null.
///
/// # Parameters
///
/// - `s`: Pointer to the string to tokenize, or null for subsequent calls.
/// - `delim`: Pointer to a null-terminated string of delimiter characters.
///
/// # Return Value
///
/// Returns a pointer to the next token, or a null pointer if there are no more tokens.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It modifies a static mutable variable for state across calls.
/// - It writes null bytes into the original string to terminate tokens.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strtok(s: *mut c_char, delim: *const c_char) -> *mut c_char {
    debug_assert!(!delim.is_null(), "strtok(): null delimiter pointer");
    debug_assert!(
        (delim as usize).is_multiple_of(align_of::<c_char>()),
        "strtok(): delimiter pointer is not properly aligned"
    );

    let mut current: *mut c_char = if !s.is_null() { s } else { LAST };

    if current.is_null() {
        return core::ptr::null_mut();
    }

    // Skip leading delimiters.
    while *current != 0 && is_delim(*current, delim) {
        current = current.add(1);
    }

    if *current == 0 {
        LAST = core::ptr::null_mut();
        return core::ptr::null_mut();
    }

    let token: *mut c_char = current;

    // Find end of token.
    while *current != 0 && !is_delim(*current, delim) {
        current = current.add(1);
    }

    if *current != 0 {
        *current = 0 as c_char;
        LAST = current.add(1);
    } else {
        LAST = core::ptr::null_mut();
    }

    token
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::strtok;
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

    #[test]
    fn test_strtok_basic() {
        let mut input: Vec<c_char> = make_c_string(b"hello world foo");
        let delim: Vec<c_char> = make_c_string(b" ");

        let t1: *mut c_char = unsafe { strtok(input.as_mut_ptr(), delim.as_ptr()) };
        assert!(!t1.is_null());
        assert_eq!(c_str_to_bytes(t1), b"hello");

        let t2: *mut c_char = unsafe { strtok(core::ptr::null_mut(), delim.as_ptr()) };
        assert!(!t2.is_null());
        assert_eq!(c_str_to_bytes(t2), b"world");

        let t3: *mut c_char = unsafe { strtok(core::ptr::null_mut(), delim.as_ptr()) };
        assert!(!t3.is_null());
        assert_eq!(c_str_to_bytes(t3), b"foo");

        let t4: *mut c_char = unsafe { strtok(core::ptr::null_mut(), delim.as_ptr()) };
        assert!(t4.is_null(), "should return null when no more tokens");
    }

    #[test]
    fn test_strtok_multiple_delimiters() {
        let mut input: Vec<c_char> = make_c_string(b"a,b;c");
        let delim: Vec<c_char> = make_c_string(b",;");

        let t1: *mut c_char = unsafe { strtok(input.as_mut_ptr(), delim.as_ptr()) };
        assert!(!t1.is_null());
        assert_eq!(c_str_to_bytes(t1), b"a");

        let t2: *mut c_char = unsafe { strtok(core::ptr::null_mut(), delim.as_ptr()) };
        assert!(!t2.is_null());
        assert_eq!(c_str_to_bytes(t2), b"b");

        let t3: *mut c_char = unsafe { strtok(core::ptr::null_mut(), delim.as_ptr()) };
        assert!(!t3.is_null());
        assert_eq!(c_str_to_bytes(t3), b"c");
    }

    #[test]
    fn test_strtok_no_tokens() {
        let mut input: Vec<c_char> = make_c_string(b",,,,");
        let delim: Vec<c_char> = make_c_string(b",");

        let t1: *mut c_char = unsafe { strtok(input.as_mut_ptr(), delim.as_ptr()) };
        assert!(t1.is_null(), "string of only delimiters should yield no tokens");
    }
}
