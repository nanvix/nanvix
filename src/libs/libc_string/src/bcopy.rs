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
/// Copies bytes in memory (legacy).
///
/// This function copies `n` bytes from memory area `src` to memory area `dest`. The memory areas
/// may overlap. Note that the parameter order is reversed from `memcpy` (src comes first).
///
/// # Parameters
///
/// - `src`: Pointer to the source memory area.
/// - `dest`: Pointer to the destination memory area.
/// - `n`: Number of bytes to copy.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It performs unchecked writes to the memory region pointed to by `dest`.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn bcopy(src: *const c_void, dest: *mut c_void, n: c_size_t) {
    debug_assert!(!src.is_null(), "bcopy(): null source pointer");
    debug_assert!(!dest.is_null(), "bcopy(): null destination pointer");
    debug_assert!(
        (src as usize).is_multiple_of(align_of::<c_uchar>()),
        "bcopy(): source pointer is not properly aligned"
    );
    debug_assert!(
        (dest as usize).is_multiple_of(align_of::<c_uchar>()),
        "bcopy(): destination pointer is not properly aligned"
    );

    let d: *mut c_uchar = dest.cast::<c_uchar>();
    let s: *const c_uchar = src.cast::<c_uchar>();
    let n: usize = n as usize;

    if n == 0 || core::ptr::eq(d.cast_const(), s) {
        return;
    }

    // Handle overlapping regions like memmove.
    if (d as usize) < (s as usize) || (d as usize) >= (s as usize + n) {
        // Forward copy.
        let mut i: usize = 0;
        while i < n {
            *d.add(i) = *s.add(i);
            i += 1;
        }
    } else {
        // Backward copy.
        let mut i: usize = n;
        while i != 0 {
            i -= 1;
            *d.add(i) = *s.add(i);
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::bcopy;

    #[test]
    fn test_bcopy_basic() {
        let src: [u8; 5] = [1, 2, 3, 4, 5];
        let mut dest: [u8; 5] = [0; 5];
        unsafe { bcopy(src.as_ptr().cast(), dest.as_mut_ptr().cast(), 5) };
        assert_eq!(dest, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_bcopy_overlapping() {
        let mut buf: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        unsafe {
            let src: *const core::ffi::c_void = buf.as_ptr().cast();
            let dest: *mut core::ffi::c_void = buf[2..].as_mut_ptr().cast();
            bcopy(src, dest, 4);
        }
        assert_eq!(buf, [1, 2, 1, 2, 3, 4, 7, 8]);
    }
}
