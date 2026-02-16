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
/// Sets bytes in memory to a specified value.
///
/// This function fills the first `len` bytes of the memory area pointed to by `ptr` with the
/// constant byte `val` (converted to an unsigned char).
///
/// # Parameters
///
/// - `ptr`: Pointer to the memory area to be filled.
/// - `val`: Value to be set. Only the least significant byte is used.
/// - `len`: Number of bytes to be set.
///
/// # Return Value
///
/// This function returns a pointer to the memory area `ptr`.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It performs unchecked writes to the memory region pointed to by `ptr`.
///
/// It is safe to call this function if and only if the following conditions are met:
/// - `ptr` points to a valid and writable memory region of at least `len` bytes.
/// - `len` does not exceed `isize::MAX`.
/// - `ptr` is properly aligned for `c_uchar` access.
///
/// Violating any of these conditions results in undefined behavior.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memset(ptr: *mut c_void, val: c_int, len: c_size_t) -> *mut c_void {
    debug_assert!(!ptr.is_null(), "memset(): null pointer");
    debug_assert!((len as usize) < isize::MAX as usize, "memset(): length too large");
    debug_assert!(
        (ptr as usize).is_multiple_of(align_of::<c_uchar>()),
        "memset(): pointer is not properly aligned"
    );

    let dst: *mut c_uchar = ptr.cast::<c_uchar>();
    let byte: c_uchar = (val & 0xFF) as c_uchar;
    let len: usize = len as usize;
    let mut i: usize = 0;
    while i < len {
        *dst.add(i) = byte;
        i += 1;
    }
    ptr
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::memset;
    use ::std::vec::Vec;
    use ::sysapi::sys_types::c_size_t;

    #[test]
    fn test_memset() {
        let mut buffer: Vec<u8> = vec![0; 10];
        unsafe {
            memset(
                buffer.as_mut_ptr().cast(),
                0xAB,
                c_size_t::try_from(buffer.len()).expect("len fits in c_size_t"),
            );
        }
        for &byte in &buffer {
            assert_eq!(byte, 0xAB);
        }
    }

    #[test]
    fn test_memset_unaligned_pointer() {
        let mut buffer: Vec<u8> = vec![0; 10];
        let ptr: *mut u8 = unsafe { buffer.as_mut_ptr().add(1) }; // Intentionally unaligned
        unsafe {
            memset(ptr.cast(), 0xCD, 5);
        }
        for &byte in &buffer[1..6] {
            assert_eq!(byte, 0xCD);
        }
    }

    #[test]
    fn test_memset_zero_length() {
        let mut buffer: Vec<u8> = vec![1, 2, 3, 4];
        let original: Vec<u8> = buffer.clone();
        unsafe {
            memset(buffer.as_mut_ptr().cast(), 0xFF, 0);
        }
        assert_eq!(buffer, original, "Buffer should be unchanged when length is zero");
    }

    #[test]
    fn test_memset_returns_pointer() {
        let mut buffer: Vec<u8> = vec![0; 4];
        let ptr: *mut core::ffi::c_void = buffer.as_mut_ptr().cast();
        let ret: *mut core::ffi::c_void = unsafe { memset(ptr, 0x11, 4) };
        assert_eq!(ret, ptr, "memset should return the original pointer");
    }

    #[test]
    fn test_memset_value_masking() {
        let mut buffer: Vec<u8> = vec![0; 8];
        // 0x1FF -> masked to 0xFF
        unsafe {
            memset(
                buffer.as_mut_ptr().cast(),
                0x1FF,
                c_size_t::try_from(buffer.len()).expect("len fits in c_size_t"),
            )
        };
        assert!(buffer.iter().all(|&b| b == 0xFF));
    }

    #[test]
    fn test_memset_partial_region() {
        let mut buffer: Vec<u8> = (0u8..16u8).collect();
        unsafe { memset(buffer.as_mut_ptr().add(4).cast(), 0x77, 8) };
        // Bytes 0..4 unchanged
        assert_eq!(&buffer[0..4], &[0, 1, 2, 3]);
        // Bytes 4..12 set to 0x77
        assert!(buffer[4..12].iter().all(|&b| b == 0x77));
        // Bytes 12..16 unchanged
        assert_eq!(&buffer[12..16], &[12, 13, 14, 15]);
    }

    #[test]
    fn test_memset_large_buffer() {
        // Large but reasonable size for unit test.
        let size: usize = 4096;
        let mut buffer: Vec<u8> = vec![0; size];
        unsafe {
            memset(
                buffer.as_mut_ptr().cast(),
                0x3C,
                c_size_t::try_from(size).expect("size fits in c_size_t"),
            )
        };
        assert!(buffer.iter().all(|&b| b == 0x3C));
    }
}
