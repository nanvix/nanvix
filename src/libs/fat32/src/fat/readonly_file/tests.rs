// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

use super::*;
use crate::{
    fat::{
        Fat,
        RawMemoryStorage,
    },
    FatResolvedPath,
};
use ::alloc::{
    string::String,
    vec,
    vec::Vec,
};

/// Size of the test image.
const IMG_SIZE: usize = 128 * 1024;

/// Contents of `hello.txt`.
const HELLO: &[u8] = b"hello readonly";

/// Size of `big.bin`, which spans several clusters of the test image.
const BIG_SIZE: usize = 20_000;

/// Helper: returns the byte that `big.bin` holds at `offset`.
fn big_byte(offset: usize) -> u8 {
    (offset % 251) as u8
}

/// Helper: builds a checked FAT-local path from a normalized test literal.
fn test_path(path: &str) -> FatResolvedPath {
    FatResolvedPath::new(String::from(path)).expect("test path should be normalized")
}

/// Helper: formats an image, populates it through the read-write path, and returns it.
///
/// The image holds `hello.txt`, an empty `empty.txt`, a multi-cluster `big.bin`, and `subdir/`.
fn make_image() -> Vec<u8> {
    let mut buf: Vec<u8> = vec![0u8; IMG_SIZE];
    let ptr: *mut u8 = buf.as_mut_ptr();

    let mut storage: RawMemoryStorage =
        unsafe { RawMemoryStorage::new(ptr, IMG_SIZE).expect("valid storage") };
    ::fatfs::format_volume(&mut storage, ::fatfs::FormatVolumeOptions::new())
        .expect("format should succeed");

    // Drop `fat` to flush all metadata before the image is used again.
    {
        let fat: Fat = unsafe { Fat::from_memory(ptr, IMG_SIZE).expect("valid fat") };
        let big: Vec<u8> = (0..BIG_SIZE).map(big_byte).collect();
        let files: [(&str, &[u8]); 3] =
            [("hello.txt", HELLO), ("empty.txt", &[]), ("big.bin", &big)];
        for (name, contents) in files {
            let mut file = fat
                .open(&test_path(name), false, true, true, false)
                .expect("create file should succeed");
            let mut written: usize = 0;
            while written < contents.len() {
                written += file
                    .write(&contents[written..])
                    .expect("write should succeed");
            }
        }
        fat.mkdir(&test_path("subdir"))
            .expect("mkdir should succeed");
    }
    buf
}

/// A read-only filesystem together with the image that backs it.
struct Fixture {
    /// The filesystem. Fields are dropped in order, and it must go before the image.
    fat: Fat,
    /// The image that backs the filesystem.
    _buf: Vec<u8>,
}

impl Fixture {
    fn new() -> Self {
        let buf: Vec<u8> = make_image();
        let fat: Fat = unsafe {
            Fat::from_memory_readonly(buf.as_ptr(), IMG_SIZE)
                .expect("readonly mount should succeed")
        };
        Self { fat, _buf: buf }
    }

    /// Opens a file of the filesystem.
    fn open(&self, path: &str) -> ReadOnlyFatFile<'_> {
        self.fat
            .open_readonly(&test_path(path))
            .expect("open_readonly should succeed")
    }
}

/// Helper: reads the file from its current position to its end, in small chunks.
fn read_to_end(file: &mut ReadOnlyFatFile<'_>) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let mut chunk: [u8; 64] = [0u8; 64];
    loop {
        let n: usize = file.read(&mut chunk).expect("read should succeed");
        if n == 0 {
            return out;
        }
        out.extend_from_slice(&chunk[..n]);
    }
}

// -- Reading -----------------------------------------------------------------

/// Tests that a file reads back what was written to it, and then reads nothing at its end.
#[test]
fn read_returns_contents_then_nothing() {
    let fixture: Fixture = Fixture::new();
    let mut file: ReadOnlyFatFile<'_> = fixture.open("hello.txt");
    assert_eq!(read_to_end(&mut file), HELLO, "contents should match what was written");

    let mut buf: [u8; 8] = [0u8; 8];
    assert_eq!(file.read(&mut buf).expect("read should succeed"), 0, "reading at the end is empty");
}

/// Tests that a file that spans several clusters reads back what was written to it.
#[test]
fn read_spans_clusters() {
    let fixture: Fixture = Fixture::new();
    let mut file: ReadOnlyFatFile<'_> = fixture.open("big.bin");
    let contents: Vec<u8> = read_to_end(&mut file);
    let expected: Vec<u8> = (0..BIG_SIZE).map(big_byte).collect();
    assert_eq!(contents, expected, "contents should match what was written");
}

/// Tests that an empty file has no size, and reads nothing.
#[test]
fn empty_file_reads_nothing() {
    let fixture: Fixture = Fixture::new();
    let mut file: ReadOnlyFatFile<'_> = fixture.open("empty.txt");
    assert!(file.is_empty().expect("is_empty should succeed"), "file should be empty");
    assert_eq!(file.len().expect("len should succeed"), 0, "size should be zero");
    assert!(read_to_end(&mut file).is_empty(), "an empty file should read nothing");
}

// -- Size --------------------------------------------------------------------

/// Tests that querying the size keeps the file position, and that the size does not change.
#[test]
fn len_keeps_position() {
    let fixture: Fixture = Fixture::new();
    let mut file: ReadOnlyFatFile<'_> = fixture.open("hello.txt");
    let mut head: [u8; 5] = [0u8; 5];
    assert_eq!(file.read(&mut head).expect("read should succeed"), head.len());
    assert_eq!(&head, b"hello");

    assert_eq!(file.len().expect("len should succeed"), HELLO.len() as u64, "size should match");
    assert_eq!(read_to_end(&mut file), b" readonly", "querying the size should keep the position");
    assert_eq!(
        file.len().expect("len should succeed"),
        HELLO.len() as u64,
        "size should be stable"
    );
    assert!(!file.is_empty().expect("is_empty should succeed"), "file should not be empty");
}

// -- Seeking -----------------------------------------------------------------

/// Tests that seeking reaches positions in different clusters, from every origin.
#[test]
fn seek_reaches_every_origin() {
    let fixture: Fixture = Fixture::new();
    let mut file: ReadOnlyFatFile<'_> = fixture.open("big.bin");
    let mut buf: [u8; 4] = [0u8; 4];

    let start: usize = 15_000;
    assert_eq!(
        file.seek(SeekFrom::Start(start as u64))
            .expect("seek should succeed"),
        start as u64
    );
    assert_eq!(file.read(&mut buf).expect("read should succeed"), buf.len());
    let expected: Vec<u8> = (start..start + buf.len()).map(big_byte).collect();
    assert_eq!(&buf[..], &expected[..], "seeking from the start should reach the offset");

    // The read moved the position forward by `buf.len()`, so this goes back to `start`.
    let back: i64 = -(buf.len() as i64);
    assert_eq!(
        file.seek(SeekFrom::Current(back))
            .expect("seek should succeed"),
        start as u64
    );
    assert_eq!(file.read(&mut buf).expect("read should succeed"), buf.len());
    assert_eq!(&buf[..], &expected[..], "seeking from the current position should work");

    let end: usize = BIG_SIZE - buf.len();
    assert_eq!(file.seek(SeekFrom::End(back)).expect("seek should succeed"), end as u64);
    assert_eq!(file.read(&mut buf).expect("read should succeed"), buf.len());
    let expected: Vec<u8> = (end..BIG_SIZE).map(big_byte).collect();
    assert_eq!(&buf[..], &expected[..], "seeking from the end should reach the offset");
}

/// Tests that seeking past the end stops at the end, and that seeking before the start fails.
#[test]
fn seek_out_of_bounds() {
    let fixture: Fixture = Fixture::new();
    let mut file: ReadOnlyFatFile<'_> = fixture.open("hello.txt");

    let past: u64 = 1_000_000;
    assert_eq!(
        file.seek(SeekFrom::Start(past))
            .expect("seek should succeed"),
        HELLO.len() as u64,
        "seeking past the end should stop at the end"
    );
    assert!(read_to_end(&mut file).is_empty(), "nothing should be left to read");

    file.seek(SeekFrom::Start(0)).expect("seek should succeed");
    assert_eq!(
        file.seek(SeekFrom::Current(-1)).unwrap_err(),
        Fat32Error::InvalidPath,
        "seeking before the start should fail"
    );
}

// -- Opening -----------------------------------------------------------------

/// Tests that opening a file that does not exist fails.
#[test]
fn open_missing_file_fails() {
    let fixture: Fixture = Fixture::new();
    let result = fixture.fat.open_readonly(&test_path("missing.txt"));
    assert_eq!(result.unwrap_err(), Fat32Error::NotFound, "a missing file should not be found");
}

/// Tests that opening a directory fails.
#[test]
fn open_directory_fails() {
    let fixture: Fixture = Fixture::new();
    let result = fixture.fat.open_readonly(&test_path("subdir"));
    assert_eq!(result.unwrap_err(), Fat32Error::InvalidPath, "a directory is not a file");
}

/// Tests that only a read-only filesystem supports opening read-only files.
#[test]
fn open_requires_readonly_filesystem() {
    let fixture: Fixture = Fixture::new();
    assert!(fixture.fat.is_readonly(), "a filesystem over read-only storage is read-only");

    let mut buf: Vec<u8> = make_image();
    let fat: Fat = unsafe { Fat::from_memory(buf.as_mut_ptr(), IMG_SIZE).expect("valid fat") };
    assert!(!fat.is_readonly(), "a filesystem over read-write storage is not read-only");
    let result = fat.open_readonly(&test_path("hello.txt"));
    assert_eq!(
        result.unwrap_err(),
        Fat32Error::NotSupported,
        "a read-write filesystem should not open read-only files"
    );
}

/// Tests that opening and reading a file does not advance the epoch of a read-only filesystem.
#[test]
fn open_keeps_epoch() {
    let fixture: Fixture = Fixture::new();
    let before: u64 = fixture.fat.epoch();
    {
        let mut file: ReadOnlyFatFile<'_> = fixture.open("big.bin");
        file.len().expect("len should succeed");
        read_to_end(&mut file);
    }
    assert_eq!(fixture.fat.epoch(), before, "reading should not change the epoch");
}
