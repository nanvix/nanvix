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
/// Compares bytes in memory.
///
/// This function compares the first `len` bytes of the memory areas pointed to by `ptr1` and
/// `ptr2`. The comparison is performed byte by byte, interpreting each byte as an unsigned char
/// (`c_uchar`).
///
/// The function stops at the first differing byte or after `len` bytes have been compared.
///
/// # Parameters
///
/// - `ptr1`: Pointer to the first memory region to compare.
/// - `ptr2`: Pointer to the second memory region to compare.
/// - `len`: Number of bytes to compare.
///
/// # Return Value
///
/// This function returns an integer less than, equal to, or greater than zero if the first
/// differing byte in `ptr1` is respectively less than, equal to, or greater than the corresponding
/// byte in `ptr2`.  If all `len` bytes are equal, returns `0`.
///
/// The exact magnitude of the non-zero return value is the arithmetic difference between the
/// first pair of differing bytes interpreted as `c_uchar` and then cast to `c_int`.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It reads from the memory regions pointed to by `ptr1` and `ptr2` without bounds checking.
///
/// It is safe to call this function if and only if all of the following conditions are met:
/// - `ptr1` points to a readable memory region of at least `len` bytes.
/// - `ptr2` points to a readable memory region of at least `len` bytes.
/// - `len` does not exceed `isize::MAX`.
/// - Both pointers are properly aligned for `c_uchar` access.
///
/// Violating any of these conditions results in undefined behavior.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcmp(ptr1: *const c_void, ptr2: *const c_void, len: c_size_t) -> c_int {
    debug_assert!(!ptr1.is_null(), "memcmp(): null pointer");
    debug_assert!(!ptr2.is_null(), "memcmp(): null pointer");
    debug_assert!((len as usize) < isize::MAX as usize, "memcmp(): length too large");
    debug_assert!(
        (ptr1 as usize).is_multiple_of(align_of::<c_uchar>()),
        "memcmp(): pointer is not properly aligned"
    );
    debug_assert!(
        (ptr2 as usize).is_multiple_of(align_of::<c_uchar>()),
        "memcmp(): pointer is not properly aligned"
    );

    let a: *const c_uchar = ptr1.cast::<c_uchar>();
    let b: *const c_uchar = ptr2.cast::<c_uchar>();
    let len: usize = len as usize;
    let mut i: usize = 0;
    while i < len {
        let va: c_uchar = *a.add(i);
        let vb: c_uchar = *b.add(i);
        if va != vb {
            return (va as c_int) - (vb as c_int);
        }
        i += 1;
    }
    0
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::memcmp;
    use ::std::vec::Vec;
    use ::sysapi::{
        ffi::c_int,
        sys_types::c_size_t,
    };

    #[test]
    fn test_memcmp_equal() {
        let size: usize = 32;
        let buf1: Vec<u8> = (0..u8::try_from(size).expect("size fits in u8")).collect();
        let buf2: Vec<u8> = (0..u8::try_from(size).expect("size fits in u8")).collect();
        let ret: c_int = unsafe {
            memcmp(
                buf1.as_ptr().cast(),
                buf2.as_ptr().cast(),
                c_size_t::try_from(size).expect("size fits in c_size_t"),
            )
        };
        assert_eq!(ret, 0, "memcmp should return 0 for identical buffers");
    }

    #[test]
    fn test_memcmp_less() {
        let mut a: Vec<u8> = vec![10, 20, 30, 40, 50];
        let mut b: Vec<u8> = a.clone();
        // Introduce first difference: a[2] < b[2].
        a[2] = 5;
        b[2] = 9;
        let ret: c_int = unsafe {
            memcmp(
                a.as_ptr().cast(),
                b.as_ptr().cast(),
                c_size_t::try_from(a.len()).expect("len fits in c_size_t"),
            )
        };
        assert_eq!(
            ret,
            (5_i32 - 9_i32) as c_int,
            "memcmp should return arithmetic difference of first differing bytes"
        );
        assert!(ret < 0, "memcmp should return a negative value when first buffer is less");
    }

    #[test]
    fn test_memcmp_greater() {
        let mut a: Vec<u8> = vec![10, 20, 30, 40, 50];
        let mut b: Vec<u8> = a.clone();
        // Introduce first difference: a[3] > b[3].
        a[3] = 100;
        b[3] = 42;
        let ret: c_int = unsafe {
            memcmp(
                a.as_ptr().cast(),
                b.as_ptr().cast(),
                c_size_t::try_from(a.len()).expect("len fits in c_size_t"),
            )
        };
        assert_eq!(
            ret,
            (100_i32 - 42_i32) as c_int,
            "memcmp should return arithmetic difference of first differing bytes"
        );
        assert!(ret > 0, "memcmp should return a positive value when first buffer is greater");
    }

    #[test]
    fn test_memcmp_zero_length() {
        let a: Vec<u8> = vec![1, 2, 3, 4];
        let b: Vec<u8> = vec![5, 6, 7, 8];
        let ret: c_int = unsafe { memcmp(a.as_ptr().cast(), b.as_ptr().cast(), 0) };
        assert_eq!(ret, 0, "memcmp should return 0 when length is zero");
    }

    #[test]
    fn test_memcmp_partial_length_ignores_later_difference() {
        let mut a: Vec<u8> = vec![1, 2, 3, 4, 5, 6];
        let mut b: Vec<u8> = a.clone();
        // Difference after the compared length.
        a[5] = 10;
        b[5] = 20;
        let len: usize = 5; // Exclude index 5.
        let ret: c_int = unsafe {
            memcmp(
                a.as_ptr().cast(),
                b.as_ptr().cast(),
                c_size_t::try_from(len).expect("len fits in c_size_t"),
            )
        };
        assert_eq!(ret, 0, "memcmp should not observe differences beyond provided length");
    }

    #[test]
    fn test_memcmp_overlapping_regions() {
        // Single buffer; compare a slice with an offset of itself.
        let buf: Vec<u8> = (0..32_u8).collect();
        // Introduce a difference at position 4 relative to start pointers.
        // We'll compare buf[0..16] with buf[1..17]; first differing byte is buf[0] vs buf[1].
        let ptr1: *const u8 = buf.as_ptr();
        let ptr2: *const u8 = unsafe { buf.as_ptr().add(1) };
        let len: usize = 16;
        let ret: c_int = unsafe {
            memcmp(ptr1.cast(), ptr2.cast(), c_size_t::try_from(len).expect("len fits in c_size_t"))
        };
        let expected: c_int = (buf[0] as c_int) - (buf[1] as c_int);
        assert_eq!(ret, expected, "memcmp should handle overlapping readable regions correctly");
    }
}
