// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use crate::fat::RawMemoryStorage;
use ::alloc::{
    vec,
    vec::Vec,
};

/// Helper: builds a checked FAT-local path from a normalized test literal.
///
/// Shared with the read-only tests, which import it from this module.
pub(super) fn test_path(path: &str) -> FatResolvedPath {
    FatResolvedPath::new(alloc::string::String::from(path)).expect("test path should be normalized")
}

/// Helper: creates formatted FAT image in a heap buffer and returns a `Fat`.
///
/// The returned `Vec<u8>` must be kept alive for the lifetime of the `Fat`.
/// Drop the `Fat` before the `Vec` to avoid use-after-free during
/// fatfs `FileSystem::drop()`.
fn make_fat(size: usize) -> (Fat, Vec<u8>) {
    let mut buf: Vec<u8> = vec![0u8; size];
    let ptr: *mut u8 = buf.as_mut_ptr();

    let mut storage: RawMemoryStorage =
        unsafe { RawMemoryStorage::new(ptr, size).expect("valid storage") };
    ::fatfs::format_volume(&mut storage, ::fatfs::FormatVolumeOptions::new())
        .expect("format should succeed");

    let fat: Fat = unsafe { Fat::from_memory(ptr, size).expect("valid fat") };
    (fat, buf)
}

/// Wrapper that ensures `Fat` is dropped before its backing buffer.
///
/// When `Fat` is dropped, fatfs `FileSystem::drop()` flushes dirty metadata
/// back to `RawMemoryStorage`. The buffer must remain valid during that flush.
/// This wrapper enforces the correct drop order.
struct FatHandle {
    /// Fat filesystem — dropped first.
    fat: core::mem::ManuallyDrop<Fat>,
    /// Backing memory — dropped second (after `fat`).
    _buf: Vec<u8>,
}

impl FatHandle {
    fn new() -> Self {
        let (fat, buf) = make_fat(IMG_SIZE);
        Self {
            fat: core::mem::ManuallyDrop::new(fat),
            _buf: buf,
        }
    }
}

impl core::ops::Deref for FatHandle {
    type Target = Fat;
    fn deref(&self) -> &Fat {
        &self.fat
    }
}

impl Drop for FatHandle {
    fn drop(&mut self) {
        // SAFETY: `fat` is dropped exactly once, before `_buf`.
        unsafe {
            core::mem::ManuallyDrop::drop(&mut self.fat);
        }
    }
}

const IMG_SIZE: usize = 128 * 1024;

// -- from_memory tests -------------------------------------------------------

/// Tests that `from_memory` rejects a null pointer.
#[test]
fn from_memory_null_ptr_fails() {
    let result: Result<Fat, Fat32Error> = unsafe { Fat::from_memory(core::ptr::null_mut(), 1024) };
    assert!(result.is_err(), "null pointer should be rejected");
}

/// Tests that `from_memory` rejects zero size.
#[test]
fn from_memory_zero_size_fails() {
    let mut buf: Vec<u8> = vec![0u8; 1024];
    let result: Result<Fat, Fat32Error> = unsafe { Fat::from_memory(buf.as_mut_ptr(), 0) };
    assert!(result.is_err(), "zero size should be rejected");
}

/// Tests that `from_memory` succeeds on a formatted image.
#[test]
fn from_memory_valid_image() {
    let _fat: FatHandle = FatHandle::new();
}

// -- open / create / read / write tests --------------------------------------

/// Tests creating a file, writing, and reading it back.
#[test]
fn write_and_read_file() {
    let fat: FatHandle = FatHandle::new();

    // Create and write.
    {
        let mut file = fat
            .open(&test_path("test.txt"), false, true, true, false)
            .expect("create should succeed");
        file.write(b"hello fat32").expect("write should succeed");
        file.flush().expect("flush should succeed");
    }

    // Read back.
    {
        let mut file = fat
            .open(&test_path("test.txt"), true, false, false, false)
            .expect("open for read should succeed");
        let mut buf: [u8; 64] = [0u8; 64];
        let n: usize = file.read(&mut buf).expect("read should succeed");
        assert_eq!(n, 11, "should read 11 bytes");
        assert_eq!(&buf[..n], b"hello fat32");
    }
}

/// Tests that opening a non-existent file without create fails.
#[test]
fn open_nonexistent_fails() {
    let fat: FatHandle = FatHandle::new();
    let result = fat.open(&test_path("nonexistent.txt"), true, false, false, false);
    assert!(result.is_err(), "opening non-existent file should fail");
}

/// Tests open with truncate.
#[test]
fn open_truncate() {
    let fat: FatHandle = FatHandle::new();

    // Write initial content.
    {
        let mut file = fat
            .open(&test_path("trunc.txt"), false, true, true, false)
            .expect("create should succeed");
        file.write(b"initial content")
            .expect("write should succeed");
        file.flush().expect("flush should succeed");
    }

    // Open with truncate.
    {
        let mut file = fat
            .open(&test_path("trunc.txt"), true, true, false, true)
            .expect("open with truncate should succeed");
        let mut buf: [u8; 64] = [0u8; 64];
        let n: usize = file.read(&mut buf).expect("read should succeed");
        assert_eq!(n, 0, "truncated file should be empty");
    }
}

/// Tests `create_new` succeeds for a new file.
#[test]
fn create_new_succeeds() {
    let fat: FatHandle = FatHandle::new();
    let _file = fat
        .create_new(&test_path("new.txt"), true, true)
        .expect("create_new should succeed");
}

/// Tests `create_new` fails if file already exists.
#[test]
fn create_new_existing_fails() {
    let fat: FatHandle = FatHandle::new();

    {
        let mut f = fat
            .open(&test_path("exists.txt"), false, true, true, false)
            .expect("create should succeed");
        f.write(b"data").expect("write should succeed");
        f.flush().expect("flush should succeed");
    }

    let result: Result<FatFile<'_>, Fat32Error> =
        fat.create_new(&test_path("exists.txt"), true, true);
    assert_eq!(
        result.unwrap_err(),
        Fat32Error::AlreadyExists,
        "create_new on existing file should return AlreadyExists"
    );
}

// -- file size tests ---------------------------------------------------------

/// Tests that the size of a file grows with writes past its end, but not with writes within it.
#[test]
fn file_len_follows_writes() {
    let fat: FatHandle = FatHandle::new();
    let mut file: FatFile<'_> = fat
        .open(&test_path("grow.txt"), true, true, true, false)
        .expect("create should succeed");
    assert_eq!(file.len().expect("len should succeed"), 0, "new file should be empty");

    file.write(b"hello").expect("write should succeed");
    assert_eq!(file.len().expect("len should succeed"), 5, "append should grow the file");
    file.write(b" world").expect("write should succeed");
    assert_eq!(file.len().expect("len should succeed"), 11, "append should grow the file");

    file.seek(::fatfs::SeekFrom::Start(0))
        .expect("seek should succeed");
    file.write(b"J").expect("write should succeed");
    assert_eq!(file.len().expect("len should succeed"), 11, "overwrite should keep the size");

    file.seek(::fatfs::SeekFrom::Start(0))
        .expect("seek should succeed");
    let mut buf: [u8; 64] = [0u8; 64];
    let n: usize = file.read(&mut buf).expect("read should succeed");
    assert_eq!(&buf[..n], b"Jello world", "read should return the written data");
}

/// Tests that the size of a file is right when first queried after a write.
#[test]
fn file_len_queried_after_write() {
    let fat: FatHandle = FatHandle::new();
    {
        let mut file: FatFile<'_> = fat
            .open(&test_path("query.txt"), false, true, true, false)
            .expect("create should succeed");
        file.write(b"hello world").expect("write should succeed");
        file.flush().expect("flush should succeed");
    }

    let mut file: FatFile<'_> = fat
        .open(&test_path("query.txt"), true, true, false, false)
        .expect("open should succeed");
    file.write(b"J").expect("write should succeed");
    assert_eq!(file.len().expect("len should succeed"), 11, "overwrite should keep the size");
}

/// Tests that the size of a file follows truncations.
#[test]
fn file_len_follows_truncation() {
    let fat: FatHandle = FatHandle::new();
    let mut file: FatFile<'_> = fat
        .open(&test_path("shrink.txt"), true, true, true, false)
        .expect("create should succeed");
    file.write(b"hello world").expect("write should succeed");
    assert_eq!(file.len().expect("len should succeed"), 11, "write should grow the file");

    file.seek(::fatfs::SeekFrom::Start(5))
        .expect("seek should succeed");
    file.truncate().expect("truncate should succeed");
    assert_eq!(file.len().expect("len should succeed"), 5, "truncate should shrink the file");

    file.seek(::fatfs::SeekFrom::Start(0))
        .expect("seek should succeed");
    file.truncate().expect("truncate should succeed");
    assert_eq!(file.len().expect("len should succeed"), 0, "truncate should empty the file");

    file.write(b"abc").expect("write should succeed");
    assert_eq!(file.len().expect("len should succeed"), 3, "write should grow the emptied file");
}

/// Tests that querying the size of a file between reads keeps the file position.
#[test]
fn file_len_keeps_position() {
    // Spans several clusters, and does not end at a cluster boundary.
    const FILE_SIZE: usize = 4 * 1024 + 7;
    const CHUNK_SIZE: usize = 300;

    let fat: FatHandle = FatHandle::new();
    let data: Vec<u8> = (0..FILE_SIZE).map(|i| (i % 251) as u8).collect();
    {
        let mut file: FatFile<'_> = fat
            .open(&test_path("big.bin"), false, true, true, false)
            .expect("create should succeed");
        let mut written: usize = 0;
        while written < data.len() {
            let n: usize = file.write(&data[written..]).expect("write should succeed");
            assert!(n > 0, "write should make progress");
            written += n;
        }
        file.flush().expect("flush should succeed");
    }

    let mut file: FatFile<'_> = fat
        .open(&test_path("big.bin"), true, false, false, false)
        .expect("open for read should succeed");
    let mut read: Vec<u8> = Vec::new();
    let mut buf: [u8; CHUNK_SIZE] = [0u8; CHUNK_SIZE];
    loop {
        let n: usize = file.read(&mut buf).expect("read should succeed");
        // The first query happens past the start of the file, and seeks to find the size.
        assert_eq!(
            file.len().expect("len should succeed"),
            FILE_SIZE as u64,
            "size should match the written data"
        );
        if n == 0 {
            break;
        }
        read.extend_from_slice(&buf[..n]);
    }
    assert_eq!(read, data, "reads should return the file contents in order");
}

// -- stat tests --------------------------------------------------------------

/// Tests stat on root directory.
#[test]
fn stat_root() {
    let fat: FatHandle = FatHandle::new();
    let info: FatStat = fat.stat(&test_path("")).expect("stat root should succeed");
    assert!(info.is_dir, "root should be a directory");
}

/// Tests stat on a file.
#[test]
fn stat_file() {
    let fat: FatHandle = FatHandle::new();

    {
        let mut f = fat
            .open(&test_path("sized.txt"), false, true, true, false)
            .expect("create should succeed");
        f.write(b"12345").expect("write should succeed");
        f.flush().expect("flush should succeed");
    }

    let info: FatStat = fat
        .stat(&test_path("sized.txt"))
        .expect("stat should succeed");
    assert!(!info.is_dir, "should not be a directory");
    assert_eq!(info.size, 5, "file size should be 5");
}

/// Tests stat on a non-existent path.
#[test]
fn stat_nonexistent() {
    let fat: FatHandle = FatHandle::new();
    let result: Result<FatStat, Fat32Error> = fat.stat(&test_path("nope.txt"));
    assert_eq!(result.unwrap_err(), Fat32Error::NotFound);
}

/// Tests lookup through a long file name's short alias.
#[test]
fn stat_short_name_alias() {
    let fat: FatHandle = FatHandle::new();
    fat.create_new(&test_path("Long File Name.txt"), true, true)
        .expect("create should succeed");

    let when: i64 = 1_718_454_600;
    fat.set_times(&test_path("LONGFI~1.TXT"), Some(when), Some(when))
        .expect("short alias should resolve");
    fat.stat(&test_path("LONGFI~1.TXT"))
        .expect("short alias should resolve");
}

/// Tests that set_times is reflected by a later stat.
#[test]
fn set_times_roundtrip() {
    let fat: FatHandle = FatHandle::new();
    fat.create_new(&test_path("t.txt"), true, true)
        .expect("create should succeed");

    // 2024-06-15T12:30:00Z.
    let when: i64 = 1_718_454_600;
    fat.set_times(&test_path("t.txt"), Some(when), Some(when))
        .expect("set_times should succeed");

    let info: FatStat = fat.stat(&test_path("t.txt")).expect("stat should succeed");
    // FAT stores modification at 2s resolution.
    assert!((info.mtime - when).abs() <= 2, "mtime should round-trip");
    // FAT stores access as a date only: truncated to midnight.
    assert_eq!(info.atime, when - when % 86_400, "atime is date-only");
}

/// Tests that set_times rejects timestamps outside the FAT range.
#[test]
fn set_times_rejects_unsupported_timestamp() {
    let fat: FatHandle = FatHandle::new();
    fat.create_new(&test_path("range.txt"), true, true)
        .expect("create should succeed");

    assert_eq!(
        fat.set_times(&test_path("range.txt"), Some(0), None),
        Err(Fat32Error::InvalidArgument)
    );
    assert_eq!(
        fat.set_times(&test_path("range.txt"), None, Some(4_354_819_200)),
        Err(Fat32Error::InvalidArgument)
    );
}

/// Tests that set_times omits the field passed as `None`.
#[test]
fn set_times_omit_leaves_mtime() {
    let fat: FatHandle = FatHandle::new();
    fat.create_new(&test_path("omit.txt"), true, true)
        .expect("create should succeed");

    let base: i64 = 1_718_454_600;
    fat.set_times(&test_path("omit.txt"), Some(base), Some(base))
        .expect("set_times should succeed");
    // Change only atime; mtime must stay put.
    fat.set_times(&test_path("omit.txt"), Some(base + 86_400), None)
        .expect("set_times should succeed");

    let info: FatStat = fat
        .stat(&test_path("omit.txt"))
        .expect("stat should succeed");
    assert!((info.mtime - base).abs() <= 2, "mtime unchanged");
}

// -- mkdir / rmdir tests -----------------------------------------------------

/// Tests creating and stat-ing a directory.
#[test]
fn mkdir_and_stat() {
    let fat: FatHandle = FatHandle::new();
    fat.mkdir(&test_path("subdir"))
        .expect("mkdir should succeed");
    let info: FatStat = fat
        .stat(&test_path("subdir"))
        .expect("stat subdir should succeed");
    assert!(info.is_dir, "should be a directory");
}

/// Tests that creating a duplicate directory fails.
#[test]
fn mkdir_duplicate_fails() {
    let fat: FatHandle = FatHandle::new();
    fat.mkdir(&test_path("dup"))
        .expect("first mkdir should succeed");
    let result: Result<(), Fat32Error> = fat.mkdir(&test_path("dup"));
    assert_eq!(result.unwrap_err(), Fat32Error::AlreadyExists);
}

/// Tests that mkdir with an empty path returns NotFound (defense-in-depth).
#[test]
fn mkdir_empty_path_fails() {
    let fat: FatHandle = FatHandle::new();
    let result: Result<(), Fat32Error> = fat.mkdir(&test_path(""));
    assert_eq!(result.unwrap_err(), Fat32Error::NotFound);
}

/// Tests removing an empty directory.
#[test]
fn rmdir_empty() {
    let fat: FatHandle = FatHandle::new();
    fat.mkdir(&test_path("torm")).expect("mkdir should succeed");
    fat.rmdir(&test_path("torm")).expect("rmdir should succeed");
    let result: Result<FatStat, Fat32Error> = fat.stat(&test_path("torm"));
    assert_eq!(result.unwrap_err(), Fat32Error::NotFound);
}

/// Tests that rmdir on a file fails.
#[test]
fn rmdir_on_file_fails() {
    let fat: FatHandle = FatHandle::new();
    {
        let mut f = fat
            .open(&test_path("file-a.txt"), false, true, true, false)
            .expect("create should succeed");
        f.write(b"x").expect("write should succeed");
        f.flush().expect("flush should succeed");
    }
    let result: Result<(), Fat32Error> = fat.rmdir(&test_path("file-a.txt"));
    assert_eq!(result.unwrap_err(), Fat32Error::NotADirectory);
}

/// Tests that rmdir with an empty path returns NotFound (defense-in-depth).
#[test]
fn rmdir_empty_path_fails() {
    let fat: FatHandle = FatHandle::new();
    let result: Result<(), Fat32Error> = fat.rmdir(&test_path(""));
    assert_eq!(result.unwrap_err(), Fat32Error::NotFound);
}

// -- unlink tests ------------------------------------------------------------

/// Tests deleting a file.
#[test]
fn unlink_file() {
    let fat: FatHandle = FatHandle::new();
    {
        let mut f = fat
            .open(&test_path("del.txt"), false, true, true, false)
            .expect("create should succeed");
        f.write(b"bye").expect("write should succeed");
        f.flush().expect("flush should succeed");
    }
    fat.unlink(&test_path("del.txt"))
        .expect("unlink should succeed");
    assert_eq!(fat.stat(&test_path("del.txt")).unwrap_err(), Fat32Error::NotFound);
}

/// Tests that unlink on a directory fails.
#[test]
fn unlink_on_dir_fails() {
    let fat: FatHandle = FatHandle::new();
    fat.mkdir(&test_path("adir")).expect("mkdir should succeed");
    let result: Result<(), Fat32Error> = fat.unlink(&test_path("adir"));
    assert_eq!(result.unwrap_err(), Fat32Error::NotAFile);
}

/// Tests that unlink with an empty path returns NotFound (defense-in-depth).
#[test]
fn unlink_empty_path_fails() {
    let fat: FatHandle = FatHandle::new();
    let result: Result<(), Fat32Error> = fat.unlink(&test_path(""));
    assert_eq!(result.unwrap_err(), Fat32Error::NotFound);
}

// -- read_dir tests ----------------------------------------------------------

/// Tests listing an empty root directory.
#[test]
fn read_dir_empty_root() {
    let fat: FatHandle = FatHandle::new();
    let entries: Vec<FatDirEntry> = fat
        .read_dir(&test_path(""))
        .expect("read_dir root should succeed");
    assert!(entries.is_empty(), "fresh root should be empty");
}

/// Tests listing root with files and directories.
#[test]
fn read_dir_with_entries() {
    let fat: FatHandle = FatHandle::new();
    {
        let mut f = fat
            .open(&test_path("f1.txt"), false, true, true, false)
            .expect("create should succeed");
        f.write(b"a").expect("write should succeed");
        f.flush().expect("flush should succeed");
    }
    fat.mkdir(&test_path("d1")).expect("mkdir should succeed");

    let entries: Vec<FatDirEntry> = fat
        .read_dir(&test_path(""))
        .expect("read_dir should succeed");
    assert_eq!(entries.len(), 2, "should have 2 entries");

    let names: alloc::vec::Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"f1.txt"), "should contain f1.txt");
    assert!(names.contains(&"d1"), "should contain d1");
}

// -- rename tests ------------------------------------------------------------

/// Tests renaming a file.
#[test]
fn rename_file() {
    let fat: FatHandle = FatHandle::new();
    {
        let mut f = fat
            .open(&test_path("old.txt"), false, true, true, false)
            .expect("create should succeed");
        f.write(b"content").expect("write should succeed");
        f.flush().expect("flush should succeed");
    }

    fat.rename(&test_path("old.txt"), &test_path("new.txt"))
        .expect("rename should succeed");
    assert_eq!(fat.stat(&test_path("old.txt")).unwrap_err(), Fat32Error::NotFound);
    let info: FatStat = fat
        .stat(&test_path("new.txt"))
        .expect("stat new.txt should succeed");
    assert!(!info.is_dir);
    assert_eq!(info.size, 7);
}

/// Tests that rename with an empty old path returns NotFound.
#[test]
fn rename_empty_old_path_fails() {
    let fat: FatHandle = FatHandle::new();
    let result: Result<(), Fat32Error> = fat.rename(&test_path(""), &test_path("new.txt"));
    assert_eq!(result.unwrap_err(), Fat32Error::NotFound);
}

/// Tests that rename with an empty new path returns NotFound.
#[test]
fn rename_empty_new_path_fails() {
    let fat: FatHandle = FatHandle::new();
    fat.mkdir(&test_path("src")).expect("mkdir should succeed");
    let result: Result<(), Fat32Error> = fat.rename(&test_path("src"), &test_path(""));
    assert_eq!(result.unwrap_err(), Fat32Error::NotFound);
}

// -- file_raw_region tests ---------------------------------------------------

/// Tests that file_raw_region returns None for non-existent files.
#[test]
fn raw_region_nonexistent() {
    let fat: FatHandle = FatHandle::new();
    assert!(fat.file_raw_region(&test_path("nope.txt")).is_none());
}

/// Tests that file_raw_region returns a valid region for a contiguous file.
#[test]
fn raw_region_contiguous_file() {
    let fat: FatHandle = FatHandle::new();
    let data: &[u8] = b"raw region data";
    {
        let mut f = fat
            .open(&test_path("raw.txt"), false, true, true, false)
            .expect("create should succeed");
        f.write(data).expect("write should succeed");
        f.flush().expect("flush should succeed");
    }

    if let Some((ptr, size)) = fat.file_raw_region(&test_path("raw.txt")) {
        assert_eq!(size, data.len(), "raw region size should match");
        let slice: &[u8] = unsafe { core::slice::from_raw_parts(ptr, size) };
        assert_eq!(slice, data, "raw region data should match");
    }
    // Note: if clusters are not contiguous, None is acceptable.
}

// -- Debug trait test --------------------------------------------------------

/// Tests that Fat implements Debug.
#[test]
fn fat_debug() {
    let handle: FatHandle = FatHandle::new();
    let fat: &Fat = &handle;
    let debug: alloc::string::String = alloc::format!("{fat:?}");
    assert!(debug.contains("Fat"), "debug should contain type name");
}
