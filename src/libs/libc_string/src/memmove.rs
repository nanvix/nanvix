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
/// Copies bytes in memory with overlapping areas.
///
/// This function copies `len` bytes from the memory area pointed to by `src` to the memory area
/// pointed to by `dest`. The memory areas may overlap: copying takes place as though the bytes in
/// `src` are first copied into a temporary array that does not overlap `src` or `dest`, and the
/// bytes are then copied from the temporary array to `dest`.
///
/// # Parameters
///
/// - `dest`: Pointer to the destination memory area where bytes will be copied.
/// - `src`: Pointer to the source memory area from which bytes will be copied.
/// - `len`: Number of bytes to copy.
///
/// # Return Value
///
/// This function returns the original destination pointer `dest`.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It performs unchecked writes to the memory region pointed to by `dest`.
///
/// It is safe to call this function if and only if the following conditions are met:
/// - `dest` points to a valid and writable memory region of at least `len` bytes.
/// - `src` points to a valid and readable memory region of at least `len` bytes.
/// - `len` does not exceed `isize::MAX`.
/// - `dest` and `src` are properly aligned for `c_uchar` access.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memmove(
    dest: *mut c_void,
    src: *const c_void,
    len: c_size_t,
) -> *mut c_void {
    debug_assert!(!dest.is_null(), "memmove(): null destination pointer");
    debug_assert!(!src.is_null(), "memmove(): null source pointer");
    debug_assert!((len as usize) < isize::MAX as usize, "memmove(): length too large");
    debug_assert!(
        (dest as usize).is_multiple_of(align_of::<c_uchar>()),
        "memmove(): destination pointer is not properly aligned"
    );

    let d: *mut c_uchar = dest.cast::<c_uchar>();
    let s: *const c_uchar = src.cast::<c_uchar>();
    let len: usize = len as usize;

    // Check for zero-length copy.
    if len == 0 {
        return dest;
    }

    // Check for self-copy.
    if core::ptr::eq(d, s) {
        return dest;
    }

    // Check whether or not to handle overlapping copy.
    if (d as usize) < (s as usize) || (d as usize) >= (s as usize + len) {
        // Non-overlapping or dest entirely before src: forward copy.
        let mut i: usize = 0;
        while i < len {
            *d.add(i) = *s.add(i);
            i += 1;
        }
    } else {
        // Overlapping and dest after src: backward copy.
        let mut i: usize = len;
        while i != 0 {
            i -= 1;
            *d.add(i) = *s.add(i);
        }
    }
    dest
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod test {
    use super::memmove;
    use ::sysapi::ffi::c_void;

    #[test]
    fn test_memmove_zero_length() {
        let mut buf: [u8; 4] = [1, 2, 3, 4];
        let original: [u8; 4] = buf;
        unsafe {
            let dst: *mut c_void = buf.as_mut_ptr().cast::<c_void>();
            let src: *const c_void = buf.as_ptr().cast::<c_void>();
            let ret: *mut c_void = memmove(dst, src, 0);
            assert_eq!(ret, dst);
        }
        assert_eq!(buf, original);
    }

    #[test]
    fn test_memmove_self_copy() {
        let mut buf: [u8; 4] = [10, 20, 30, 40];
        unsafe {
            let dst: *mut c_void = buf.as_mut_ptr().cast::<c_void>();
            let src: *const c_void = buf.as_ptr().cast::<c_void>();
            memmove(dst, src, u32::try_from(buf.len()).expect("buf len fits in u32"));
        }
        assert_eq!(buf, [10, 20, 30, 40]);
    }

    #[test]
    fn test_memmove_non_overlapping() {
        let mut buf: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
        unsafe {
            let dst: *mut c_void = buf[4..].as_mut_ptr().cast::<c_void>();
            let src: *const c_void = buf[..4].as_ptr().cast::<c_void>();
            memmove(dst, src, 4);
        }
        assert_eq!(buf, [0, 1, 2, 3, 0, 1, 2, 3]);
    }

    #[test]
    fn test_memmove_overlapping_forward() {
        // Destination starts before source: forward copy is fine.
        let mut buf: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        unsafe {
            let dst: *mut c_void = buf.as_mut_ptr().cast::<c_void>(); // start at index 0
            let src: *const c_void = buf[2..].as_ptr().cast::<c_void>(); // start at index 2
            memmove(dst, src, 4); // copy 4 bytes: 3,4,5,6 into start
        }
        assert_eq!(buf, [3, 4, 5, 6, 5, 6, 7, 8]);
    }

    #[test]
    fn test_memmove_overlapping_backward() {
        // Destination starts after source: must copy backwards to preserve data.
        let mut buf: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        unsafe {
            let dst: *mut c_void = buf[2..].as_mut_ptr().cast::<c_void>(); // index 2
            let src: *const c_void = buf.as_ptr().cast::<c_void>(); // index 0
            memmove(dst, src, 4); // copy 1,2,3,4 into positions 2..5
        }
        assert_eq!(buf, [1, 2, 1, 2, 3, 4, 7, 8]);
    }
}
