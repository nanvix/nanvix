// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::cast_sign_loss)]

//==================================================================================================
// Imports
//==================================================================================================

use ::core::mem::{
    align_of,
    size_of,
};
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
/// Copies bytes from one memory region to another.
///
/// This function copies exactly `len` bytes from the memory region pointed to by `src` to the
/// memory region pointed to by `dest`.
///
/// Behavior is undefined if the source and destination memory regions overlap. Use `memmove()`
/// (if available) when regions may overlap.
///
/// # Parameters
///
/// - `dest`: Destination pointer where bytes will be written.
/// - `src`: Source pointer from which bytes will be read.
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
/// - `dest` and `src` are properly aligned for `c_uchar` access
/// - `dest` and `src` do not point to overlapping memory regions.
///
/// Violating any of these conditions results in undefined behavior.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcpy(
    dest: *mut c_void,
    src: *const c_void,
    len: c_size_t,
) -> *mut c_void {
    debug_assert!(!dest.is_null(), "memcpy(): null destination pointer");
    debug_assert!(!src.is_null(), "memcpy(): null source pointer");
    debug_assert!((len as usize) < isize::MAX as usize, "memcpy(): length too large");
    debug_assert!(
        (dest as usize).is_multiple_of(align_of::<c_uchar>()),
        "memcpy(): destination pointer is not properly aligned"
    );
    debug_assert!(
        (src as usize).is_multiple_of(align_of::<c_uchar>()),
        "memcpy(): source pointer is not properly aligned"
    );

    const SMALL_COPY_THRESHOLD: usize = 16;
    const WORD_SIZE: usize = size_of::<usize>();

    let d: *mut c_uchar = dest.cast::<c_uchar>();
    let s: *const c_uchar = src.cast::<c_uchar>();
    let s_addr: usize = s as usize;
    let d_addr: usize = d as usize;
    let len: usize = len as usize;

    // Ensure that memory regions do not overlap.
    if len > 0 {
        debug_assert!(
            !((d_addr >= s_addr && d_addr < s_addr + len)
                || (s_addr >= d_addr && s_addr < d_addr + len)),
            "memcpy(): overlapping memory regions"
        );
    }

    // Trivial.
    if len == 0 {
        return dest;
    }

    // Small copies: simple byte loop.
    if len < SMALL_COPY_THRESHOLD {
        copy_bytes(d, s, len);
        return dest;
    }

    // Check alignments.
    let same_alignment: bool = { ((s_addr ^ d_addr) & (WORD_SIZE - 1)) == 0 };

    // If alignments differ, fallback to byte copy (cannot safely build aligned word copies).
    if !same_alignment {
        copy_bytes(d, s, len);
        return dest;
    }

    // Align destination (and thus source) to word boundary.
    let mut offset: usize = 0;
    while ((d_addr + offset) & (WORD_SIZE - 1)) != 0 && offset < len {
        *d.add(offset) = *s.add(offset);
        offset += 1;
    }

    // Word-sized bulk copy.
    let mut i: usize = offset;
    while i + WORD_SIZE <= len {
        *d.add(i).cast::<usize>() = *s.add(i).cast::<usize>();
        i += WORD_SIZE;
    }

    // Copy tail bytes.
    if i < len {
        copy_bytes(d.add(i), s.add(i), len - i);
    }

    dest
}

///
/// # Description
///
/// Performs a simple byte-by-byte copy from `src` to `dst` for `len` bytes.
///
/// This is a small internal helper extracted from `memcpy()` to centralize the trivial byte loop
/// used in multiple fallback and tail scenarios.
///
/// # Parameters
///
/// - `dst`: Destination pointer (must be valid for writes of `len` bytes).
/// - `src`: Source pointer (must be valid for reads of `len` bytes).
/// - `len`: Number of bytes to copy.
///
/// # Safety
///
/// The caller must uphold the same safety invariants required by `memcpy()`.
///
#[inline(always)]
unsafe fn copy_bytes(dst: *mut c_uchar, src: *const c_uchar, len: usize) {
    let mut i: usize = 0;
    while i < len {
        *dst.add(i) = *src.add(i);
        i += 1;
    }
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::memcpy;
    use ::std::vec::Vec;
    use ::sysapi::sys_types::c_size_t;

    #[test]
    fn test_memcpy() {
        let size: usize = 10;
        let src: Vec<u8> = (0..size)
            .map(|i| u8::try_from(i).expect("index fits in u8"))
            .collect();
        let mut dst: Vec<u8> = vec![0; size];

        unsafe {
            memcpy(
                dst.as_mut_ptr().cast(),
                src.as_ptr().cast(),
                c_size_t::try_from(size).expect("size fits in c_size_t"),
            );
        }

        // Verify that destination buffer matches source buffer.
        assert_eq!(dst, src);
    }

    #[test]
    fn test_memcpy_unaligned_pointer() {
        let size: usize = 10;
        let mut src: Vec<u8> = vec![0; size + 1]; // Extra byte for unalignment
        let mut dst: Vec<u8> = vec![0xFF; size + 1]; // Extra byte for unalignment

        // Initialize source buffer with known pattern (starting at index 1).
        for (i, byte) in src[1..=size].iter_mut().enumerate() {
            *byte = u8::try_from(i).expect("index fits in u8");
        }

        let src_ptr: *const u8 = unsafe { src.as_ptr().add(1) }; // Intentionally unaligned
        let dst_ptr: *mut u8 = unsafe { dst.as_mut_ptr().add(1) }; // Intentionally unaligned

        unsafe {
            memcpy(
                dst_ptr.cast(),
                src_ptr.cast(),
                c_size_t::try_from(size).expect("size fits in c_size_t"),
            );
        }

        // Verify that destination buffer matches source buffer.
        assert_eq!(dst[1..=size], src[1..=size]);
    }

    #[test]
    fn test_memcpy_zero_length() {
        let size: usize = 10;
        let src: Vec<u8> = (0..size)
            .map(|i| u8::try_from(i).expect("index fits in u8"))
            .collect();
        let mut dst: Vec<u8> = vec![0xFF; size];

        unsafe {
            memcpy(dst.as_mut_ptr().cast(), src.as_ptr().cast(), 0);
        }

        // Verify that destination buffer remains unchanged.
        assert!(dst.iter().all(|&b| b == 0xFF));
    }

    #[test]
    fn test_memcpy_returns_pointer() {
        let size: usize = 10;
        let src: Vec<u8> = vec![0; size];
        let mut dst: Vec<u8> = vec![0; size];
        let dst_ptr: *mut core::ffi::c_void = dst.as_mut_ptr().cast();
        let ret: *mut core::ffi::c_void = unsafe {
            memcpy(
                dst_ptr,
                src.as_ptr().cast(),
                c_size_t::try_from(size).expect("size fits in c_size_t"),
            )
        };
        assert_eq!(ret, dst_ptr, "memcpy should return the original destination pointer");
    }

    #[test]
    fn test_memcpy_partial_buffer() {
        let size: usize = 64;
        let src: Vec<u8> = (0..u8::try_from(size).expect("size fits in u8")).collect();
        let mut dst: Vec<u8> = vec![0xEE; size];
        let copy_offset: usize = 16;
        let copy_len: usize = 32;
        unsafe {
            memcpy(
                dst.as_mut_ptr().add(copy_offset).cast(),
                src.as_ptr().add(copy_offset).cast(),
                c_size_t::try_from(copy_len).expect("copy_len fits in c_size_t"),
            );
        }
        // Bytes before the copied range should remain unchanged.
        assert!(
            dst[..copy_offset].iter().all(|&b| b == 0xEE),
            "bytes before copied region were modified"
        );
        // Copied range should match source.
        assert_eq!(
            dst[copy_offset..copy_offset + copy_len],
            src[copy_offset..copy_offset + copy_len]
        );
        // Bytes after the copied range should remain unchanged.
        assert!(
            dst[copy_offset + copy_len..].iter().all(|&b| b == 0xEE),
            "bytes after copied region were modified"
        );
    }

    #[test]
    fn test_memcpy_large_aligned() {
        let size: usize = 4096;
        let src: Vec<u8> = (0..size)
            .map(|i| u8::try_from(i & 0xFF).expect("masked value fits in u8"))
            .collect();
        let mut dst: Vec<u8> = vec![0u8; size];
        unsafe {
            memcpy(
                dst.as_mut_ptr().cast(),
                src.as_ptr().cast(),
                c_size_t::try_from(size).expect("size fits in c_size_t"),
            );
        }
        assert_eq!(dst, src);
    }

    #[test]
    fn test_memcpy_misaligned_same_offset() {
        // Create buffers with identical misalignment to trigger word path with alignment fix-up.
        let size: usize = 257;
        let copy_len: usize = size - 1;
        let src: Vec<u8> = (0..size)
            .map(|i| u8::try_from(i & 0xFF).expect("masked value fits in u8"))
            .collect();
        let mut dst: Vec<u8> = vec![0u8; size];
        let src_ptr: *const u8 = unsafe { src.as_ptr().add(1) }; // Misaligned by +1.
        let dst_ptr: *mut u8 = unsafe { dst.as_mut_ptr().add(1) }; // Same misalignment.
        unsafe {
            memcpy(
                dst_ptr.cast(),
                src_ptr.cast(),
                c_size_t::try_from(copy_len).expect("size fits in c_size_t"),
            );
        }
        let dst_slice: &[u8] = unsafe { ::core::slice::from_raw_parts(dst_ptr, copy_len) };
        let src_slice: &[u8] = unsafe { ::core::slice::from_raw_parts(src_ptr, copy_len) };
        assert_eq!(dst_slice, src_slice);
    }

    #[test]
    fn test_memcpy_misaligned_different_offset() {
        // Different relative alignment forces byte fallback path.
        let size: usize = 513;
        let copy_len: usize = size - 2;
        let src: Vec<u8> = (0..size)
            .map(|i| u8::try_from(i & 0xFF).expect("masked value fits in u8"))
            .collect();
        let mut dst: Vec<u8> = vec![0u8; size];
        let src_ptr: *const u8 = unsafe { src.as_ptr().add(1) }; // +1
        let dst_ptr: *mut u8 = unsafe { dst.as_mut_ptr().add(2) }; // +2 (different alignment)
        unsafe {
            memcpy(
                dst_ptr.cast(),
                src_ptr.cast(),
                c_size_t::try_from(copy_len).expect("size fits in c_size_t"),
            );
        }
        let dst_slice: &[u8] = unsafe { ::core::slice::from_raw_parts(dst_ptr, copy_len) };
        let src_slice: &[u8] = unsafe { ::core::slice::from_raw_parts(src_ptr, copy_len) };
        assert_eq!(dst_slice, src_slice);
    }
}
