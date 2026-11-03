// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::alloc::{
    vec,
    vec::Vec,
};

/// Helper: creates a `ReadOnlyMemoryStorage` backed by a Vec.
/// Returns the storage and the backing buffer (which must be kept alive).
fn make_readonly_storage(size: usize) -> (ReadOnlyMemoryStorage, Vec<u8>) {
    let buf: Vec<u8> = vec![0u8; size];
    let ptr: *const u8 = buf.as_ptr();
    // SAFETY: ptr points to `buf` which is valid for `size` bytes.
    let storage: ReadOnlyMemoryStorage =
        unsafe { ReadOnlyMemoryStorage::new(ptr, size).expect("valid storage") };
    (storage, buf)
}

// -- Constructor tests -------------------------------------------------------

/// Tests that null pointer is rejected.
#[test]
fn new_rejects_null_pointer() {
    let result: Result<ReadOnlyMemoryStorage, _> =
        unsafe { ReadOnlyMemoryStorage::new(core::ptr::null(), 1024) };
    assert_eq!(result.unwrap_err(), Fat32Error::InvalidArgument, "null pointer should be rejected");
}

/// Tests that zero size is rejected.
#[test]
fn new_rejects_zero_size() {
    let buf: [u8; 1] = [0];
    let result: Result<ReadOnlyMemoryStorage, _> =
        unsafe { ReadOnlyMemoryStorage::new(buf.as_ptr(), 0) };
    assert_eq!(result.unwrap_err(), Fat32Error::InvalidArgument, "zero size should be rejected");
}

/// Tests successful construction.
#[test]
fn new_succeeds_with_valid_args() {
    let (storage, _buf) = make_readonly_storage(64);
    assert_eq!(storage.remaining(), 64, "remaining should equal size at position 0");
}

// -- Read tests --------------------------------------------------------------

/// Tests reading data from the backing buffer.
#[test]
fn read_returns_data() {
    let buf: Vec<u8> = vec![0xBB; 16];
    let ptr: *const u8 = buf.as_ptr();
    let mut storage: ReadOnlyMemoryStorage =
        unsafe { ReadOnlyMemoryStorage::new(ptr, 16).expect("valid storage") };

    let mut read_buf: [u8; 4] = [0; 4];
    let n: usize = storage.read(&mut read_buf).expect("read should succeed");
    assert_eq!(n, 4, "should read 4 bytes");
    assert_eq!(read_buf, [0xBB; 4], "should read the correct data");
}

/// Tests reading an empty buffer.
#[test]
fn read_empty_buffer_returns_zero() {
    let (mut storage, _buf) = make_readonly_storage(16);
    let mut empty: [u8; 0] = [];
    let n: usize = storage.read(&mut empty).expect("read should succeed");
    assert_eq!(n, 0, "reading empty buffer should return 0");
}

/// Tests that read at end of storage returns 0.
#[test]
fn read_at_end_returns_zero() {
    let (mut storage, _buf) = make_readonly_storage(8);
    storage.seek(SeekFrom::End(0)).expect("seek to end");

    let mut read_buf: [u8; 4] = [0; 4];
    let n: usize = storage.read(&mut read_buf).expect("read should succeed");
    assert_eq!(n, 0, "reading at end should return 0");
}

// -- Write tests (always fail) -----------------------------------------------

/// Tests that write always returns ReadOnly error.
#[test]
fn write_returns_read_only_error() {
    let (mut storage, _buf) = make_readonly_storage(32);
    let result = storage.write(b"hello");
    assert_eq!(result.unwrap_err(), MemoryIoError::ReadOnly, "write should return ReadOnly error");
}

/// Tests that flush succeeds (no-op for read-only storage).
#[test]
fn flush_succeeds() {
    let (mut storage, _buf) = make_readonly_storage(8);
    storage
        .flush()
        .expect("flush should always succeed on read-only storage");
}

// -- Seek tests --------------------------------------------------------------

/// Tests seek from start.
#[test]
fn seek_from_start() {
    let (mut storage, _buf) = make_readonly_storage(64);
    let pos: u64 = storage
        .seek(SeekFrom::Start(10))
        .expect("seek should succeed");
    assert_eq!(pos, 10, "position should be 10");
}

/// Tests seek from end.
#[test]
fn seek_from_end() {
    let (mut storage, _buf) = make_readonly_storage(64);
    let pos: u64 = storage
        .seek(SeekFrom::End(-4))
        .expect("seek should succeed");
    assert_eq!(pos, 60, "position should be 60");
}

/// Tests that seeking past the end fails.
#[test]
fn seek_past_end_fails() {
    let (mut storage, _buf) = make_readonly_storage(64);
    let result = storage.seek(SeekFrom::Start(65));
    assert_eq!(result.unwrap_err(), MemoryIoError::OutOfBounds, "should fail with OutOfBounds");
}

// -- Debug trait test --------------------------------------------------------

/// Tests that Debug formatting includes type name.
#[test]
fn debug_format() {
    let (storage, _buf) = make_readonly_storage(32);
    let debug_str: alloc::string::String = alloc::format!("{storage:?}");
    assert!(debug_str.contains("ReadOnlyMemoryStorage"), "debug output should contain type name");
}
