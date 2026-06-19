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
/// Gets the length of a complementary substring.
///
/// This function calculates the length of the initial segment of the string `s` which consists
/// entirely of bytes NOT in `reject`.
///
/// # Parameters
///
/// - `s`: Pointer to the null-terminated string to search.
/// - `reject`: Pointer to a null-terminated string of rejected bytes.
///
/// # Return Value
///
/// Returns the number of bytes in the initial segment of `s` which are not in `reject`.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It reads from the memory regions pointed to by `s` and `reject` without bounds checking.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strcspn(s: *const c_char, reject: *const c_char) -> c_size_t {
    debug_assert!(!s.is_null(), "strcspn(): null pointer");
    debug_assert!(!reject.is_null(), "strcspn(): null reject pointer");
    debug_assert!(
        (s as usize).is_multiple_of(align_of::<c_char>()),
        "strcspn(): pointer is not properly aligned"
    );
    debug_assert!(
        (reject as usize).is_multiple_of(align_of::<c_char>()),
        "strcspn(): reject pointer is not properly aligned"
    );

    let mut count: c_size_t = 0;
    while *s.add(count as usize) != 0 {
        let ch: c_char = *s.add(count as usize);
        let mut rejected: bool = false;
        let mut r: usize = 0;
        while *reject.add(r) != 0 {
            if ch == *reject.add(r) {
                rejected = true;
                break;
            }
            r += 1;
        }
        if rejected {
            break;
        }
        count += 1;
    }

    count
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::strcspn;
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
    fn test_strcspn_all_rejected() {
        let s: Vec<c_char> = make_c_string(b"abc");
        let reject: Vec<c_char> = make_c_string(b"abc");
        let ret: c_size_t = unsafe { strcspn(s.as_ptr(), reject.as_ptr()) };
        assert_eq!(ret as usize, 0, "first char is rejected");
    }

    #[test]
    fn test_strcspn_none_rejected() {
        let s: Vec<c_char> = make_c_string(b"abc");
        let reject: Vec<c_char> = make_c_string(b"xyz");
        let ret: c_size_t = unsafe { strcspn(s.as_ptr(), reject.as_ptr()) };
        assert_eq!(ret as usize, 3, "no chars are rejected");
    }

    #[test]
    fn test_strcspn_partial() {
        let s: Vec<c_char> = make_c_string(b"abcxyz");
        let reject: Vec<c_char> = make_c_string(b"xyz");
        let ret: c_size_t = unsafe { strcspn(s.as_ptr(), reject.as_ptr()) };
        assert_eq!(ret as usize, 3, "first 3 chars are not rejected");
    }
}
