// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    ffi::{
        c_int,
        c_void,
    },
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Compares the first `n` bytes of the memory areas `s1` and `s2` (legacy BSD interface).
///
/// # Returns
///
/// Zero if the byte ranges are equal, and non-zero otherwise.
///
/// # Safety
///
/// Both `s1` and `s2` must point to at least `n` readable bytes.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/bcmp.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn bcmp(s1: *const c_void, s2: *const c_void, n: c_size_t) -> c_int {
    let a: *const u8 = s1.cast::<u8>();
    let b: *const u8 = s2.cast::<u8>();
    let mut i: c_size_t = 0;
    while i < n {
        if *a.add(i as usize) != *b.add(i as usize) {
            return 1;
        }
        i += 1;
    }
    0
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::bcmp;
    use ::std::vec::Vec;
    use ::sysapi::ffi::c_int;

    #[test]
    fn test_bcmp_equal() {
        let a: Vec<u8> = vec![1, 2, 3, 4, 5];
        let b: Vec<u8> = vec![1, 2, 3, 4, 5];
        let ret: c_int = unsafe { bcmp(a.as_ptr().cast(), b.as_ptr().cast(), 5) };
        assert_eq!(ret, 0, "bcmp should return 0 for equal byte ranges");
    }

    #[test]
    fn test_bcmp_differ() {
        let a: Vec<u8> = vec![1, 2, 3, 4, 5];
        let b: Vec<u8> = vec![1, 2, 0, 4, 5];
        let ret: c_int = unsafe { bcmp(a.as_ptr().cast(), b.as_ptr().cast(), 5) };
        assert_ne!(ret, 0, "bcmp should return non-zero for differing byte ranges");
    }

    #[test]
    fn test_bcmp_zero_length() {
        let a: Vec<u8> = vec![1, 2, 3];
        let b: Vec<u8> = vec![4, 5, 6];
        let ret: c_int = unsafe { bcmp(a.as_ptr().cast(), b.as_ptr().cast(), 0) };
        assert_eq!(ret, 0, "bcmp with zero length should return 0");
    }
}
