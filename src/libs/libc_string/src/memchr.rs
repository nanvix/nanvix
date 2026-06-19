// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::mem::align_of;
use ::sysapi::{
    ffi::{
        c_int,
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
/// Scans memory for a byte.
///
/// This function scans the initial `n` bytes of the memory area pointed to by `s` for the first
/// instance of `c`. Both `c` and the bytes of the memory area pointed to by `s` are interpreted as
/// unsigned chars.
///
/// # Parameters
///
/// - `s`: Pointer to the memory area to search.
/// - `c`: Byte value to search for.
/// - `n`: Number of bytes to search.
///
/// # Return Value
///
/// Returns a pointer to the matching byte, or a null pointer if the byte is not found within the
/// first `n` bytes.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It reads from the memory region pointed to by `s` without bounds checking.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn memchr(s: *const c_void, c: c_int, n: c_size_t) -> *mut c_void {
    debug_assert!(!s.is_null(), "memchr(): null pointer");
    debug_assert!(
        (s as usize).is_multiple_of(align_of::<c_uchar>()),
        "memchr(): pointer is not properly aligned"
    );

    let target: c_uchar = c.to_le_bytes()[0];
    let p: *const c_uchar = s.cast::<c_uchar>();
    let n: usize = n as usize;
    let mut i: usize = 0;
    while i < n {
        if *p.add(i) == target {
            return s.add(i).cast_mut();
        }
        i += 1;
    }

    core::ptr::null_mut()
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::memchr;
    use ::std::vec::Vec;
    use ::sysapi::ffi::c_int;

    #[test]
    fn test_memchr_found() {
        let buf: Vec<u8> = vec![1, 2, 3, 4, 5];
        let ret: *mut core::ffi::c_void = unsafe { memchr(buf.as_ptr().cast(), 3 as c_int, 5) };
        assert!(!ret.is_null(), "memchr should find the byte");
        let offset: usize = (ret as usize) - (buf.as_ptr() as usize);
        assert_eq!(offset, 2, "byte 3 is at index 2");
    }

    #[test]
    fn test_memchr_not_found() {
        let buf: Vec<u8> = vec![1, 2, 3, 4, 5];
        let ret: *mut core::ffi::c_void = unsafe { memchr(buf.as_ptr().cast(), 9 as c_int, 5) };
        assert!(ret.is_null(), "memchr should return null when byte not found");
    }

    #[test]
    fn test_memchr_zero_length() {
        let buf: Vec<u8> = vec![1, 2, 3];
        let ret: *mut core::ffi::c_void = unsafe { memchr(buf.as_ptr().cast(), 1 as c_int, 0) };
        assert!(ret.is_null(), "memchr should return null with zero length");
    }
}
