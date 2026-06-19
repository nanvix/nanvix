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
/// This function locates in the null-terminated string referenced by `*stringp` the first
/// occurrence of any character in the string `delim` and replaces it with a null byte. The
/// location of the next character after the delimiter is stored in `*stringp`. The original
/// value of `*stringp` is returned.
///
/// If no delimiter is found, `*stringp` is set to null and the token (the entire string) is
/// returned. If `*stringp` is initially null, null is returned.
///
/// # Parameters
///
/// - `stringp`: Pointer to a pointer to the string being tokenized.
/// - `delim`: Pointer to a null-terminated string of delimiter characters.
///
/// # Return Value
///
/// Returns a pointer to the original value of `*stringp` (the token), or a null pointer if
/// `*stringp` is null.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It modifies the string in place by writing null bytes.
/// - It modifies `*stringp` to advance past the delimiter.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strsep(stringp: *mut *mut c_char, delim: *const c_char) -> *mut c_char {
    debug_assert!(!stringp.is_null(), "strsep(): null stringp pointer");
    debug_assert!(!delim.is_null(), "strsep(): null delimiter pointer");
    debug_assert!(
        (delim as usize).is_multiple_of(align_of::<c_char>()),
        "strsep(): delimiter pointer is not properly aligned"
    );

    let s: *mut c_char = *stringp;
    if s.is_null() {
        return core::ptr::null_mut();
    }

    let token: *mut c_char = s;
    let mut i: usize = 0;
    while *s.add(i) != 0 {
        if is_delim(*s.add(i), delim) {
            *s.add(i) = 0 as c_char;
            *stringp = s.add(i + 1);
            return token;
        }
        i += 1;
    }

    // No delimiter found.
    *stringp = core::ptr::null_mut();
    token
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::strsep;
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
    fn test_strsep_basic() {
        let mut input: Vec<c_char> = make_c_string(b"hello,world");
        let delim: Vec<c_char> = make_c_string(b",");
        let mut ptr: *mut c_char = input.as_mut_ptr();

        let t1: *mut c_char = unsafe { strsep(&mut ptr, delim.as_ptr()) };
        assert!(!t1.is_null());
        assert_eq!(c_str_to_bytes(t1), b"hello");

        let t2: *mut c_char = unsafe { strsep(&mut ptr, delim.as_ptr()) };
        assert!(!t2.is_null());
        assert_eq!(c_str_to_bytes(t2), b"world");

        let t3: *mut c_char = unsafe { strsep(&mut ptr, delim.as_ptr()) };
        assert!(t3.is_null(), "should return null when no more tokens");
    }

    #[test]
    fn test_strsep_multiple_tokens() {
        let mut input: Vec<c_char> = make_c_string(b"a;b;c");
        let delim: Vec<c_char> = make_c_string(b";");
        let mut ptr: *mut c_char = input.as_mut_ptr();

        let t1: *mut c_char = unsafe { strsep(&mut ptr, delim.as_ptr()) };
        assert_eq!(c_str_to_bytes(t1), b"a");

        let t2: *mut c_char = unsafe { strsep(&mut ptr, delim.as_ptr()) };
        assert_eq!(c_str_to_bytes(t2), b"b");

        let t3: *mut c_char = unsafe { strsep(&mut ptr, delim.as_ptr()) };
        assert_eq!(c_str_to_bytes(t3), b"c");

        let t4: *mut c_char = unsafe { strsep(&mut ptr, delim.as_ptr()) };
        assert!(t4.is_null());
    }
}
