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
/// Scans memory for a byte in reverse.
///
/// This function scans the initial `n` bytes of the memory area pointed to by `s` for the last
/// instance of `c`. Both `c` and the bytes of the memory area are interpreted as unsigned chars.
///
/// # Parameters
///
/// - `s`: Pointer to the memory area to search.
/// - `c`: Byte value to search for.
/// - `n`: Number of bytes to search.
///
/// # Return Value
///
/// Returns a pointer to the last matching byte, or a null pointer if the byte is not found within
/// the first `n` bytes.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It reads from the memory region pointed to by `s` without bounds checking.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn memrchr(s: *const c_void, c: c_int, n: c_size_t) -> *mut c_void {
    debug_assert!(!s.is_null(), "memrchr(): null pointer");
    debug_assert!(
        (s as usize).is_multiple_of(align_of::<c_uchar>()),
        "memrchr(): pointer is not properly aligned"
    );

    let target: c_uchar = c.to_le_bytes()[0];
    let p: *const c_uchar = s.cast::<c_uchar>();
    let n: usize = n as usize;
    let mut i: usize = n;
    while i > 0 {
        i -= 1;
        if *p.add(i) == target {
            return s.add(i).cast_mut();
        }
    }

    core::ptr::null_mut()
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::memrchr;
    use ::std::vec::Vec;
    use ::sysapi::ffi::c_int;

    #[test]
    fn test_memrchr_found_at_end() {
        let buf: Vec<u8> = vec![1, 2, 3, 2, 5];
        let ret: *mut core::ffi::c_void = unsafe { memrchr(buf.as_ptr().cast(), 2 as c_int, 5) };
        assert!(!ret.is_null(), "memrchr should find the byte");
        let offset: usize = (ret as usize) - (buf.as_ptr() as usize);
        assert_eq!(offset, 3, "last 2 is at index 3");
    }

    #[test]
    fn test_memrchr_found_at_start() {
        let buf: Vec<u8> = vec![7, 1, 2, 3, 4];
        let ret: *mut core::ffi::c_void = unsafe { memrchr(buf.as_ptr().cast(), 7 as c_int, 5) };
        assert!(!ret.is_null(), "memrchr should find the byte");
        let offset: usize = (ret as usize) - (buf.as_ptr() as usize);
        assert_eq!(offset, 0, "7 is at index 0");
    }

    #[test]
    fn test_memrchr_not_found() {
        let buf: Vec<u8> = vec![1, 2, 3, 4, 5];
        let ret: *mut core::ffi::c_void = unsafe { memrchr(buf.as_ptr().cast(), 9 as c_int, 5) };
        assert!(ret.is_null(), "memrchr should return null when byte not found");
    }
}
