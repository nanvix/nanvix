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
/// Gets the length of a prefix substring.
///
/// This function calculates the length of the initial segment of the string `s` which consists
/// entirely of bytes in `accept`.
///
/// # Parameters
///
/// - `s`: Pointer to the null-terminated string to search.
/// - `accept`: Pointer to a null-terminated string of accepted bytes.
///
/// # Return Value
///
/// Returns the number of bytes in the initial segment of `s` which consist only of bytes from
/// `accept`.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It reads from the memory regions pointed to by `s` and `accept` without bounds checking.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strspn(s: *const c_char, accept: *const c_char) -> c_size_t {
    debug_assert!(!s.is_null(), "strspn(): null pointer");
    debug_assert!(!accept.is_null(), "strspn(): null accept pointer");
    debug_assert!(
        (s as usize).is_multiple_of(align_of::<c_char>()),
        "strspn(): pointer is not properly aligned"
    );
    debug_assert!(
        (accept as usize).is_multiple_of(align_of::<c_char>()),
        "strspn(): accept pointer is not properly aligned"
    );

    let mut count: c_size_t = 0;
    while *s.add(count as usize) != 0 {
        let ch: c_char = *s.add(count as usize);
        let mut found: bool = false;
        let mut a: usize = 0;
        while *accept.add(a) != 0 {
            if ch == *accept.add(a) {
                found = true;
                break;
            }
            a += 1;
        }
        if !found {
            break;
        }
        count += 1;
    }

    count
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::strspn;
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
    fn test_strspn_all_match() {
        let s: Vec<c_char> = make_c_string(b"aabbcc");
        let accept: Vec<c_char> = make_c_string(b"abc");
        let ret: c_size_t = unsafe { strspn(s.as_ptr(), accept.as_ptr()) };
        assert_eq!(ret as usize, 6, "all chars are in accept");
    }

    #[test]
    fn test_strspn_none_match() {
        let s: Vec<c_char> = make_c_string(b"xyz");
        let accept: Vec<c_char> = make_c_string(b"abc");
        let ret: c_size_t = unsafe { strspn(s.as_ptr(), accept.as_ptr()) };
        assert_eq!(ret as usize, 0, "no chars match");
    }

    #[test]
    fn test_strspn_partial_match() {
        let s: Vec<c_char> = make_c_string(b"aabxyz");
        let accept: Vec<c_char> = make_c_string(b"ab");
        let ret: c_size_t = unsafe { strspn(s.as_ptr(), accept.as_ptr()) };
        assert_eq!(ret as usize, 3, "first 3 chars match");
    }
}
