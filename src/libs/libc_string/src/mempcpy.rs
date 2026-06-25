// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::cast_sign_loss)]

//==================================================================================================
// Imports
//==================================================================================================

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
/// Copies bytes from one memory region to another and returns a pointer past the end.
///
/// This function copies exactly `len` bytes from the memory region pointed to by `src` to the
/// memory region pointed to by `dest`, exactly like `memcpy()`, but returns a pointer to the byte
/// following the last written byte (`dest + len`) rather than `dest`. It is a GNU extension that is
/// convenient for chaining successive copies.
///
/// Behavior is undefined if the source and destination memory regions overlap.
///
/// # Parameters
///
/// - `dest`: Destination pointer where bytes will be written.
/// - `src`: Source pointer from which bytes will be read.
/// - `len`: Number of bytes to copy.
///
/// # Return Value
///
/// This function returns `dest + len` (a pointer to the byte after the last one written).
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
/// - `dest` and `src` do not point to overlapping memory regions.
///
/// Violating any of these conditions results in undefined behavior.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn mempcpy(
    dest: *mut c_void,
    src: *const c_void,
    len: c_size_t,
) -> *mut c_void {
    debug_assert!(!dest.is_null(), "mempcpy(): null destination pointer");
    debug_assert!(!src.is_null(), "mempcpy(): null source pointer");

    crate::memcpy::memcpy(dest, src, len);
    dest.cast::<c_uchar>().add(len as usize).cast::<c_void>()
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::mempcpy;
    use ::std::vec::Vec;
    use ::sysapi::sys_types::c_size_t;

    #[test]
    fn test_mempcpy_copies_bytes() {
        let size: usize = 10;
        let src: Vec<u8> = (0..size)
            .map(|i| u8::try_from(i).expect("index fits in u8"))
            .collect();
        let mut dst: Vec<u8> = vec![0; size];

        unsafe {
            mempcpy(
                dst.as_mut_ptr().cast(),
                src.as_ptr().cast(),
                c_size_t::try_from(size).expect("size fits in c_size_t"),
            );
        }

        assert_eq!(dst, src);
    }

    #[test]
    fn test_mempcpy_returns_end_pointer() {
        let size: usize = 10;
        let src: Vec<u8> = vec![0; size];
        let mut dst: Vec<u8> = vec![0; size];
        let dst_ptr: *mut core::ffi::c_void = dst.as_mut_ptr().cast();
        let ret: *mut core::ffi::c_void = unsafe {
            mempcpy(
                dst_ptr,
                src.as_ptr().cast(),
                c_size_t::try_from(size).expect("size fits in c_size_t"),
            )
        };
        let expected: usize = (dst_ptr as usize) + size;
        assert_eq!(ret as usize, expected, "mempcpy should return dest + len");
    }

    #[test]
    fn test_mempcpy_zero_length() {
        let size: usize = 4;
        let src: Vec<u8> = vec![1; size];
        let mut dst: Vec<u8> = vec![0xFF; size];
        let dst_ptr: *mut core::ffi::c_void = dst.as_mut_ptr().cast();
        let ret: *mut core::ffi::c_void = unsafe { mempcpy(dst_ptr, src.as_ptr().cast(), 0) };
        assert_eq!(ret, dst_ptr, "mempcpy(.., 0) should return dest unchanged");
        assert!(dst.iter().all(|&b| b == 0xFF), "destination must be untouched");
    }
}
