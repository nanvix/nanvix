// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
