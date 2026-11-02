// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::alloc::{
    vec,
    vec::Vec,
};

/// Helper: creates a `RawMemoryStorage` backed by a mutable Vec.
/// Returns the storage and the backing buffer (which must be kept alive).
fn make_storage(size: usize) -> (RawMemoryStorage, Vec<u8>) {
    let mut buf: Vec<u8> = vec![0u8; size];
    let ptr: *mut u8 = buf.as_mut_ptr();
    // SAFETY: ptr points to `buf` which is valid for `size` bytes.
    let storage: RawMemoryStorage =
        unsafe { RawMemoryStorage::new(ptr, size).expect("valid storage") };
    (storage, buf)
}

// -- Constructor tests -------------------------------------------------------

/// Tests that null pointer is rejected.
#[test]
fn new_rejects_null_pointer() {
    let result: Result<RawMemoryStorage, _> =
        unsafe { RawMemoryStorage::new(core::ptr::null_mut(), 1024) };
    assert_eq!(result.unwrap_err(), Fat32Error::InvalidArgument, "null pointer should be rejected");
}

/// Tests that zero size is rejected.
#[test]
fn new_rejects_zero_size() {
    let mut buf: [u8; 1] = [0];
    let result: Result<RawMemoryStorage, _> = unsafe { RawMemoryStorage::new(buf.as_mut_ptr(), 0) };
    assert_eq!(result.unwrap_err(), Fat32Error::InvalidArgument, "zero size should be rejected");
}

/// Tests successful construction.
#[test]
fn new_succeeds_with_valid_args() {
    let (storage, _buf) = make_storage(64);
    assert_eq!(storage.remaining(), 64, "remaining should equal size at position 0");
}

// -- Read tests --------------------------------------------------------------

/// Tests reading data that was written to the backing buffer.
#[test]
fn read_returns_data() {
    let mut buf: Vec<u8> = vec![0xAA; 16];
    let ptr: *mut u8 = buf.as_mut_ptr();
    let mut storage: RawMemoryStorage =
        unsafe { RawMemoryStorage::new(ptr, 16).expect("valid storage") };

    let mut read_buf: [u8; 4] = [0; 4];
    let n: usize = storage.read(&mut read_buf).expect("read should succeed");
    assert_eq!(n, 4, "should read 4 bytes");
    assert_eq!(read_buf, [0xAA; 4], "should read the correct data");
}

/// Tests reading an empty buffer.
#[test]
fn read_empty_buffer_returns_zero() {
    let (mut storage, _buf) = make_storage(16);
    let mut empty: [u8; 0] = [];
    let n: usize = storage.read(&mut empty).expect("read should succeed");
    assert_eq!(n, 0, "reading empty buffer should return 0");
}

/// Tests that read at end of storage returns 0.
#[test]
fn read_at_end_returns_zero() {
    let (mut storage, _buf) = make_storage(8);
    storage.seek(SeekFrom::End(0)).expect("seek to end");

    let mut read_buf: [u8; 4] = [0; 4];
    let n: usize = storage.read(&mut read_buf).expect("read should succeed");
    assert_eq!(n, 0, "reading at end should return 0");
}

/// Tests partial read when buffer is larger than remaining data.
#[test]
fn read_partial_at_boundary() {
    let (mut storage, _buf) = make_storage(8);
    storage
        .seek(SeekFrom::Start(6))
        .expect("seek should succeed");

    let mut read_buf: [u8; 8] = [0; 8];
    let n: usize = storage.read(&mut read_buf).expect("read should succeed");
    assert_eq!(n, 2, "should only read 2 remaining bytes");
}

// -- Write tests -------------------------------------------------------------

/// Tests basic write and read back.
#[test]
fn write_and_read_back() {
    let (mut storage, _buf) = make_storage(32);
    let data: &[u8] = b"hello";
    let n: usize = storage.write(data).expect("write should succeed");
    assert_eq!(n, 5, "should write 5 bytes");

    storage
        .seek(SeekFrom::Start(0))
        .expect("seek should succeed");
    let mut read_buf: [u8; 5] = [0; 5];
    storage.read(&mut read_buf).expect("read should succeed");
    assert_eq!(&read_buf, b"hello", "read back should match written data");
}

/// Tests writing empty buffer.
#[test]
fn write_empty_buffer_returns_zero() {
    let (mut storage, _buf) = make_storage(16);
    let n: usize = storage.write(&[]).expect("write should succeed");
    assert_eq!(n, 0, "writing empty buffer should return 0");
}

/// Tests partial write when buffer is larger than remaining space.
#[test]
fn write_partial_at_boundary() {
    let (mut storage, _buf) = make_storage(8);
    storage
        .seek(SeekFrom::Start(6))
        .expect("seek should succeed");

    let data: [u8; 8] = [0xFF; 8];
    let n: usize = storage.write(&data).expect("write should succeed");
    assert_eq!(n, 2, "should only write 2 remaining bytes");
}

/// Tests that flush succeeds (no-op for memory).
#[test]
fn flush_succeeds() {
    let (mut storage, _buf) = make_storage(8);
    storage.flush().expect("flush should always succeed");
}

// -- Seek tests --------------------------------------------------------------

/// Tests seek from start.
#[test]
fn seek_from_start() {
    let (mut storage, _buf) = make_storage(64);
    let pos: u64 = storage
        .seek(SeekFrom::Start(10))
        .expect("seek should succeed");
    assert_eq!(pos, 10, "position should be 10");
}

/// Tests seek from end.
#[test]
fn seek_from_end() {
    let (mut storage, _buf) = make_storage(64);
    let pos: u64 = storage
        .seek(SeekFrom::End(-4))
        .expect("seek should succeed");
    assert_eq!(pos, 60, "position should be 60");
}

/// Tests seek from current position.
#[test]
fn seek_from_current() {
    let (mut storage, _buf) = make_storage(64);
    storage
        .seek(SeekFrom::Start(20))
        .expect("seek should succeed");
    let pos: u64 = storage
        .seek(SeekFrom::Current(5))
        .expect("seek should succeed");
    assert_eq!(pos, 25, "position should be 25");
}

/// Tests seek from current with negative offset.
#[test]
fn seek_current_negative() {
    let (mut storage, _buf) = make_storage(64);
    storage
        .seek(SeekFrom::Start(20))
        .expect("seek should succeed");
    let pos: u64 = storage
        .seek(SeekFrom::Current(-10))
        .expect("seek should succeed");
    assert_eq!(pos, 10, "position should be 10");
}

/// Tests that seeking to the exact end (size) succeeds.
#[test]
fn seek_to_exact_end() {
    let (mut storage, _buf) = make_storage(64);
    let pos: u64 = storage
        .seek(SeekFrom::Start(64))
        .expect("seek to exact end should succeed");
    assert_eq!(pos, 64, "position should be 64");
}

/// Tests that seeking past the end fails.
#[test]
fn seek_past_end_fails() {
    let (mut storage, _buf) = make_storage(64);
    let result = storage.seek(SeekFrom::Start(65));
    assert_eq!(result.unwrap_err(), MemoryIoError::OutOfBounds, "should fail with OutOfBounds");
}

/// Tests that seeking to negative position fails.
#[test]
fn seek_negative_fails() {
    let (mut storage, _buf) = make_storage(64);
    let result = storage.seek(SeekFrom::Current(-1));
    assert_eq!(result.unwrap_err(), MemoryIoError::InvalidSeek, "should fail with InvalidSeek");
}

/// Tests seek to position 0 (Current(0)) returns current position.
#[test]
fn seek_current_zero_returns_position() {
    let (mut storage, _buf) = make_storage(64);
    storage
        .seek(SeekFrom::Start(30))
        .expect("seek should succeed");
    let pos: u64 = storage
        .seek(SeekFrom::Current(0))
        .expect("seek should succeed");
    assert_eq!(pos, 30, "Current(0) should return current position");
}

/// Tests that End(0) returns the size.
#[test]
fn seek_end_zero() {
    let (mut storage, _buf) = make_storage(64);
    let pos: u64 = storage.seek(SeekFrom::End(0)).expect("seek should succeed");
    assert_eq!(pos, 64, "End(0) should return size");
}

// -- Debug trait test --------------------------------------------------------

/// Tests that Debug formatting includes field names.
#[test]
fn debug_format() {
    let (storage, _buf) = make_storage(32);
    let debug_str: alloc::string::String = alloc::format!("{storage:?}");
    assert!(debug_str.contains("RawMemoryStorage"), "debug output should contain type name");
    assert!(debug_str.contains("size"), "debug output should contain 'size'");
    assert!(debug_str.contains("position"), "debug output should contain 'position'");
}
