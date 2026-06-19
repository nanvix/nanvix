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
/// Copies bytes from one memory region to another, stopping after the first occurrence of byte `c`
/// or after `n` bytes have been copied, whichever comes first.
///
/// # Parameters
///
/// - `dest`: Destination pointer where bytes will be written.
/// - `src`: Source pointer from which bytes will be read.
/// - `c`: Byte value to stop copying after (interpreted as `unsigned char`).
/// - `n`: Maximum number of bytes to copy.
///
/// # Return Value
///
/// Returns a pointer to the byte in `dest` immediately after the copy of `c`, if `c` was found
/// in the first `n` bytes of `src`. Returns a null pointer if `c` was not found.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It performs unchecked writes to the memory region pointed to by `dest`.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn memccpy(
    dest: *mut c_void,
    src: *const c_void,
    c: c_int,
    n: c_size_t,
) -> *mut c_void {
    debug_assert!(!dest.is_null(), "memccpy(): null destination pointer");
    debug_assert!(!src.is_null(), "memccpy(): null source pointer");
    debug_assert!(
        (dest as usize).is_multiple_of(align_of::<c_uchar>()),
        "memccpy(): destination pointer is not properly aligned"
    );
    debug_assert!(
        (src as usize).is_multiple_of(align_of::<c_uchar>()),
        "memccpy(): source pointer is not properly aligned"
    );

    let target: c_uchar = c.to_le_bytes()[0];
    let d: *mut c_uchar = dest.cast::<c_uchar>();
    let s: *const c_uchar = src.cast::<c_uchar>();

    let mut i: c_size_t = 0;
    while i < n {
        let byte: c_uchar = *s.add(i as usize);
        *d.add(i as usize) = byte;
        if byte == target {
            return d.add(i as usize + 1).cast::<c_void>();
        }
        i += 1;
    }

    core::ptr::null_mut()
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::memccpy;
    use ::std::vec::Vec;
    use ::sysapi::{
        ffi::c_int,
        sys_types::c_size_t,
    };

    #[test]
    fn test_memccpy_char_found() {
        let src: Vec<u8> = vec![1, 2, 3, 4, 5];
        let mut dst: Vec<u8> = vec![0; 5];
        let ret: *mut core::ffi::c_void = unsafe {
            memccpy(
                dst.as_mut_ptr().cast(),
                src.as_ptr().cast(),
                3 as c_int,
                c_size_t::try_from(src.len()).expect("len fits in c_size_t"),
            )
        };
        assert!(!ret.is_null(), "memccpy should return non-null when char is found");
        // Bytes up to and including the target should be copied.
        assert_eq!(dst[0], 1);
        assert_eq!(dst[1], 2);
        assert_eq!(dst[2], 3);
        // Return pointer should be one past the copied target byte.
        let expected_offset: usize = 3;
        let actual_offset: usize = (ret as usize) - (dst.as_ptr() as usize);
        assert_eq!(actual_offset, expected_offset, "return pointer should be one past target");
    }

    #[test]
    fn test_memccpy_char_not_found() {
        let src: Vec<u8> = vec![1, 2, 3, 4, 5];
        let mut dst: Vec<u8> = vec![0; 5];
        let ret: *mut core::ffi::c_void = unsafe {
            memccpy(
                dst.as_mut_ptr().cast(),
                src.as_ptr().cast(),
                9 as c_int,
                c_size_t::try_from(src.len()).expect("len fits in c_size_t"),
            )
        };
        assert!(ret.is_null(), "memccpy should return null when char is not found");
        // All bytes should still be copied.
        assert_eq!(dst, src);
    }

    #[test]
    fn test_memccpy_zero_length() {
        let src: Vec<u8> = vec![1, 2, 3];
        let mut dst: Vec<u8> = vec![0xFF; 3];
        let ret: *mut core::ffi::c_void =
            unsafe { memccpy(dst.as_mut_ptr().cast(), src.as_ptr().cast(), 1 as c_int, 0) };
        assert!(ret.is_null(), "memccpy with zero length should return null");
        assert!(dst.iter().all(|&b| b == 0xFF), "destination should be unchanged");
    }
}
