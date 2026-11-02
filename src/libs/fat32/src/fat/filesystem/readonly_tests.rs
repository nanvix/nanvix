// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::{
    tests::test_path,
    *,
};
use crate::fat::RawMemoryStorage;
use ::alloc::{
    vec,
    vec::Vec,
};

const IMG_SIZE: usize = 128 * 1024;

/// Helper: creates a formatted FAT image with a pre-populated file and directory,
/// then re-opens it as read-only via `Fat::from_memory_readonly`.
///
/// Returns the read-only `Fat` and the backing buffer (which must outlive `Fat`).
fn make_readonly_fat() -> (Fat, Vec<u8>) {
    let mut buf: Vec<u8> = vec![0u8; IMG_SIZE];
    let ptr: *mut u8 = buf.as_mut_ptr();

    // Format and seed with content using the read-write path.
    {
        let mut storage: RawMemoryStorage =
            unsafe { RawMemoryStorage::new(ptr, IMG_SIZE).expect("valid storage") };
        ::fatfs::format_volume(&mut storage, ::fatfs::FormatVolumeOptions::new())
            .expect("format should succeed");

        let fat: Fat = unsafe { Fat::from_memory(ptr, IMG_SIZE).expect("valid fat") };
        {
            let mut file = fat
                .open(&test_path("hello.txt"), false, true, true, false)
                .expect("create file should succeed");
            file.write(b"hello readonly").expect("write should succeed");
            file.flush().expect("flush should succeed");
        }
        fat.mkdir(&test_path("subdir"))
            .expect("mkdir should succeed");
        // Drop `fat` to flush all metadata before re-opening as read-only.
    }

    // Re-open as read-only.
    let fat: Fat = unsafe {
        Fat::from_memory_readonly(buf.as_ptr(), IMG_SIZE).expect("readonly mount should succeed")
    };
    (fat, buf)
}

/// Wrapper that ensures `Fat` is dropped before its backing buffer.
struct ReadOnlyFatHandle {
    fat: core::mem::ManuallyDrop<Fat>,
    _buf: Vec<u8>,
}

impl ReadOnlyFatHandle {
    fn new() -> Self {
        let (fat, buf) = make_readonly_fat();
        Self {
            fat: core::mem::ManuallyDrop::new(fat),
            _buf: buf,
        }
    }
}

impl core::ops::Deref for ReadOnlyFatHandle {
    type Target = Fat;
    fn deref(&self) -> &Fat {
        &self.fat
    }
}

impl Drop for ReadOnlyFatHandle {
    fn drop(&mut self) {
        // SAFETY: `fat` is dropped exactly once, before `_buf`.
        unsafe {
            core::mem::ManuallyDrop::drop(&mut self.fat);
        }
    }
}

// -- Constructor tests -------------------------------------------------------

/// Tests that `from_memory_readonly` rejects a null pointer.
#[test]
fn from_memory_readonly_null_ptr_fails() {
    let result: Result<Fat, Fat32Error> =
        unsafe { Fat::from_memory_readonly(core::ptr::null(), 1024) };
    assert!(result.is_err(), "null pointer should be rejected");
}

/// Tests that `from_memory_readonly` rejects zero size.
#[test]
fn from_memory_readonly_zero_size_fails() {
    let buf: [u8; 1024] = [0; 1024];
    let result: Result<Fat, Fat32Error> = unsafe { Fat::from_memory_readonly(buf.as_ptr(), 0) };
    assert!(result.is_err(), "zero size should be rejected");
}

/// Tests that `from_memory_readonly` succeeds on a formatted image.
#[test]
fn from_memory_readonly_valid_image() {
    let _fat: ReadOnlyFatHandle = ReadOnlyFatHandle::new();
}

// -- Read-only read operations -----------------------------------------------

/// Tests that stat works on a read-only filesystem.
#[test]
fn readonly_stat_file() {
    let fat: ReadOnlyFatHandle = ReadOnlyFatHandle::new();
    let info: FatStat = fat
        .stat(&test_path("hello.txt"))
        .expect("stat should succeed");
    assert!(!info.is_dir, "should not be a directory");
    assert_eq!(info.size, 14, "file size should match written content");
}

/// Tests that stat on a directory works on a read-only filesystem.
#[test]
fn readonly_stat_dir() {
    let fat: ReadOnlyFatHandle = ReadOnlyFatHandle::new();
    let info: FatStat = fat
        .stat(&test_path("subdir"))
        .expect("stat subdir should succeed");
    assert!(info.is_dir, "should be a directory");
}

/// Tests that stat on root works on a read-only filesystem.
#[test]
fn readonly_stat_root() {
    let fat: ReadOnlyFatHandle = ReadOnlyFatHandle::new();
    let info: FatStat = fat.stat(&test_path("")).expect("stat root should succeed");
    assert!(info.is_dir, "root should be a directory");
}

/// Tests reading a file on a read-only filesystem.
#[test]
fn readonly_read_file() {
    let fat: ReadOnlyFatHandle = ReadOnlyFatHandle::new();
    let mut file = fat
        .open(&test_path("hello.txt"), true, false, false, false)
        .expect("open for read should succeed");
    let mut buf: [u8; 64] = [0u8; 64];
    let n: usize = file.read(&mut buf).expect("read should succeed");
    assert_eq!(&buf[..n], b"hello readonly");
}

/// Tests read_dir on a read-only filesystem.
#[test]
fn readonly_read_dir() {
    let fat: ReadOnlyFatHandle = ReadOnlyFatHandle::new();
    let entries: Vec<FatDirEntry> = fat
        .read_dir(&test_path(""))
        .expect("read_dir should succeed");
    assert_eq!(entries.len(), 2, "should have 2 entries (file + dir)");

    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"hello.txt"), "should contain hello.txt");
    assert!(names.contains(&"subdir"), "should contain subdir");
}

// -- Read-only write guards --------------------------------------------------

/// Tests that omitting both timestamps does not resolve the path.
#[test]
fn readonly_set_times_omit_succeeds() {
    let fat: ReadOnlyFatHandle = ReadOnlyFatHandle::new();
    fat.set_times(&test_path("missing.txt"), None, None)
        .expect("omitting timestamps should succeed");
}

/// Tests that open with write flag returns ReadOnly on a read-only filesystem.
#[test]
fn readonly_open_write_fails() {
    let fat: ReadOnlyFatHandle = ReadOnlyFatHandle::new();
    let result = fat.open(&test_path("hello.txt"), true, true, false, false);
    assert_eq!(result.unwrap_err(), Fat32Error::ReadOnly, "open with write should return ReadOnly");
}

/// Tests that open with create flag returns ReadOnly on a read-only filesystem.
#[test]
fn readonly_open_create_fails() {
    let fat: ReadOnlyFatHandle = ReadOnlyFatHandle::new();
    let result = fat.open(&test_path("new.txt"), true, false, true, false);
    assert_eq!(
        result.unwrap_err(),
        Fat32Error::ReadOnly,
        "open with create should return ReadOnly"
    );
}

/// Tests that open with truncate flag returns ReadOnly on a read-only filesystem.
#[test]
fn readonly_open_truncate_fails() {
    let fat: ReadOnlyFatHandle = ReadOnlyFatHandle::new();
    let result = fat.open(&test_path("hello.txt"), true, false, false, true);
    assert_eq!(
        result.unwrap_err(),
        Fat32Error::ReadOnly,
        "open with truncate should return ReadOnly"
    );
}

/// Tests that create_new returns ReadOnly on a read-only filesystem.
#[test]
fn readonly_create_new_fails() {
    let fat: ReadOnlyFatHandle = ReadOnlyFatHandle::new();
    let result = fat.create_new(&test_path("brand-new.txt"), true, true);
    assert_eq!(result.unwrap_err(), Fat32Error::ReadOnly, "create_new should return ReadOnly");
}

/// Tests that mkdir returns ReadOnly on a read-only filesystem.
#[test]
fn readonly_mkdir_fails() {
    let fat: ReadOnlyFatHandle = ReadOnlyFatHandle::new();
    let result: Result<(), Fat32Error> = fat.mkdir(&test_path("newdir"));
    assert_eq!(result.unwrap_err(), Fat32Error::ReadOnly, "mkdir should return ReadOnly");
}

/// Tests that rmdir returns ReadOnly on a read-only filesystem.
#[test]
fn readonly_rmdir_fails() {
    let fat: ReadOnlyFatHandle = ReadOnlyFatHandle::new();
    let result: Result<(), Fat32Error> = fat.rmdir(&test_path("subdir"));
    assert_eq!(result.unwrap_err(), Fat32Error::ReadOnly, "rmdir should return ReadOnly");
}

/// Tests that unlink returns ReadOnly on a read-only filesystem.
#[test]
fn readonly_unlink_fails() {
    let fat: ReadOnlyFatHandle = ReadOnlyFatHandle::new();
    let result: Result<(), Fat32Error> = fat.unlink(&test_path("hello.txt"));
    assert_eq!(result.unwrap_err(), Fat32Error::ReadOnly, "unlink should return ReadOnly");
}

/// Tests that rename returns ReadOnly on a read-only filesystem.
#[test]
fn readonly_rename_fails() {
    let fat: ReadOnlyFatHandle = ReadOnlyFatHandle::new();
    let result: Result<(), Fat32Error> =
        fat.rename(&test_path("hello.txt"), &test_path("renamed.txt"));
    assert_eq!(result.unwrap_err(), Fat32Error::ReadOnly, "rename should return ReadOnly");
}
