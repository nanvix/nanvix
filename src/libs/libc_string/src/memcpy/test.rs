// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
    assert_eq!(dst[copy_offset..copy_offset + copy_len], src[copy_offset..copy_offset + copy_len]);
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
