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
    ffi::{
        c_uchar,
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
/// Zeros a byte string (legacy).
///
/// This function writes `n` zeroed bytes to the memory area pointed to by `s`. This is equivalent
/// to `memset(s, 0, n)`.
///
/// # Parameters
///
/// - `s`: Pointer to the memory area to zero.
/// - `n`: Number of bytes to zero.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It performs unchecked writes to the memory region pointed to by `s`.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn bzero(s: *mut c_void, n: c_size_t) {
    debug_assert!(!s.is_null(), "bzero(): null pointer");
    debug_assert!(
        (s as usize).is_multiple_of(align_of::<c_uchar>()),
        "bzero(): pointer is not properly aligned"
    );

    let dst: *mut c_uchar = s.cast::<c_uchar>();
    let n: usize = n as usize;
    let mut i: usize = 0;
    while i < n {
        *dst.add(i) = 0;
        i += 1;
    }
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::bzero;
    use ::std::vec::Vec;

    #[test]
    fn test_bzero_basic() {
        let mut buf: Vec<u8> = vec![0xFF; 10];
        unsafe { bzero(buf.as_mut_ptr().cast(), 10) };
        assert!(buf.iter().all(|&b| b == 0), "all bytes should be zero");
    }
}
