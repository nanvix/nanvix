// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::alloc::{
    vec,
    vec::Vec,
};
use ::fat32::{
    FatFile,
    RawMemoryStorage,
};

/// Size of the image that backs the filesystem of a test mount.
const IMAGE_SIZE: usize = 64 * 1024;

/// A timestamp within the FAT range, used to set the times of files (2024-06-15T12:30:00Z).
const FILE_TIME: i64 = 1_718_454_600;

/// A mount that owns the memory that its filesystem lives in.
struct TestMount {
    /// Mount under test. Fields are dropped in declaration order, so the mount, which flushes its
    /// filesystem to the image when it is dropped, goes before the image does.
    mount: Mount,
    /// Memory that backs the filesystem.
    _image: Vec<u8>,
}

impl TestMount {
    /// Creates a mount of a filesystem that holds the file `file.txt`, with the contents `hello`,
    /// and the empty directory `dir`.
    fn new(readonly: bool) -> Self {
        let mut image: Vec<u8> = vec![0u8; IMAGE_SIZE];
        let ptr: *mut u8 = image.as_mut_ptr();

        let mut storage: RawMemoryStorage =
            unsafe { RawMemoryStorage::new(ptr, IMAGE_SIZE).expect("valid storage") };
        ::fatfs::format_volume(&mut storage, ::fatfs::FormatVolumeOptions::new())
            .expect("format should succeed");
        {
            // Dropping the filesystem flushes it to the image.
            let fat: Fat = unsafe { Fat::from_memory(ptr, IMAGE_SIZE).expect("valid fat") };
            let mut file = fat
                .create_new(&path("file.txt"), false, true)
                .expect("create should succeed");
            file.write(b"hello").expect("write should succeed");
            fat.mkdir(&path("dir")).expect("mkdir should succeed");
        }

        let fat: Fat = if readonly {
            unsafe { Fat::from_memory_readonly(ptr, IMAGE_SIZE) }
        } else {
            unsafe { Fat::from_memory(ptr, IMAGE_SIZE) }
        }
        .expect("valid fat");
        let mount: Mount = Mount::new(String::from("/mnt"), fat, readonly).expect("valid mount");
        Self {
            mount,
            _image: image,
        }
    }
}

/// Builds a checked path, relative to the mount, from a normalized test literal.
fn path(path: &str) -> FatResolvedPath {
    FatResolvedPath::new(String::from(path)).expect("test path should be normalized")
}

/// Asks the filesystem of a mount for the metadata of a path, without going through the cache.
fn uncached(mount: &Mount, path: &FatResolvedPath) -> Result<Stat, Fat32Error> {
    mount
        .fat()
        .stat(path)
        .map(|info| Stat::new(info.size, info.is_dir, info.atime, info.mtime, info.ctime))
}

/// Asserts that a mount answers for each path like its filesystem does.
fn assert_coherent(mount: &mut Mount, paths: &[&str]) {
    for name in paths {
        let checked: FatResolvedPath = path(name);
        let expected: Result<Stat, Fat32Error> = uncached(mount, &checked);
        assert_eq!(mount.stat(&checked), expected, "{name:?} should be answered like the filesystem");
    }
}

/// Asks a mount for the size of a file.
fn size(mount: &mut Mount, name: &str) -> Result<u64, Fat32Error> {
    mount.stat(&path(name)).map(|stat| stat.size())
}

/// Tests that a mount reports the metadata of paths that exist and the absence of paths that do not,
/// the first time they are queried and when the cache answers.
#[test]
fn stat_reports_metadata_and_absence() {
    let mut test: TestMount = TestMount::new(false);

    for _ in 0..2 {
        let file: Stat = test.mount.stat(&path("file.txt")).expect("file should exist");
        assert_eq!(file.size(), 5, "file size should be reported");
        assert!(!file.is_dir(), "file should not be a directory");
        let dir: Stat = test.mount.stat(&path("dir")).expect("directory should exist");
        assert!(dir.is_dir(), "directory should be reported as such");
        assert_eq!(
            test.mount.stat(&path("missing")),
            Err(Fat32Error::NotFound),
            "missing path should not be found"
        );
        assert_eq!(
            test.mount.stat(&path("dir/missing")),
            Err(Fat32Error::NotFound),
            "missing path in a directory should not be found"
        );
        assert_coherent(
            &mut test.mount,
            &["file.txt", "dir", "missing", "dir/missing", "file.txt/child"],
        );
    }
}

/// Tests that a path that was found to be missing is found once it is created.
#[test]
fn stat_follows_creation() {
    let mut test: TestMount = TestMount::new(false);
    assert_eq!(size(&mut test.mount, "new.txt"), Err(Fat32Error::NotFound));
    assert_eq!(size(&mut test.mount, "other.txt"), Err(Fat32Error::NotFound));

    test.mount
        .fat()
        .create_new(&path("new.txt"), false, true)
        .expect("create should succeed");
    test.mount
        .fat()
        .open(&path("other.txt"), false, true, true, false)
        .expect("open that creates should succeed");

    assert_eq!(size(&mut test.mount, "new.txt"), Ok(0), "created file should be found");
    assert_eq!(size(&mut test.mount, "other.txt"), Ok(0), "created file should be found");
    assert_coherent(&mut test.mount, &["new.txt", "other.txt"]);
}

/// Tests that the metadata of a file follows the data that is written to, and truncated from, it
/// through its handles, which do not go through the mount.
#[test]
fn stat_follows_handle_writes() {
    let mut test: TestMount = TestMount::new(false);
    assert_eq!(size(&mut test.mount, "file.txt"), Ok(5));

    {
        let mut file: FatFile<'_> = test
            .mount
            .fat()
            .open(&path("file.txt"), false, true, false, false)
            .expect("open should succeed");
        file.seek(::fatfs::SeekFrom::End(0))
            .expect("seek should succeed");
        file.write(b" world").expect("write should succeed");
    }
    assert_eq!(size(&mut test.mount, "file.txt"), Ok(11), "write should grow the file");

    {
        let mut file: FatFile<'_> = test
            .mount
            .fat()
            .open(&path("file.txt"), false, true, false, false)
            .expect("open should succeed");
        file.seek(::fatfs::SeekFrom::Start(3))
            .expect("seek should succeed");
        file.truncate().expect("truncate should succeed");
    }
    assert_eq!(size(&mut test.mount, "file.txt"), Ok(3), "truncate should shrink the file");
    assert_coherent(&mut test.mount, &["file.txt"]);
}

/// Tests that the metadata of a file follows its truncation when it is opened.
#[test]
fn stat_follows_truncating_open() {
    let mut test: TestMount = TestMount::new(false);
    assert_eq!(size(&mut test.mount, "file.txt"), Ok(5));

    test.mount
        .fat()
        .open(&path("file.txt"), false, true, false, true)
        .expect("open with truncate should succeed");

    assert_eq!(size(&mut test.mount, "file.txt"), Ok(0), "open should truncate the file");
    assert_coherent(&mut test.mount, &["file.txt"]);
}

/// Tests that the metadata of the names involved in a rename follows it.
#[test]
fn stat_follows_rename() {
    let mut test: TestMount = TestMount::new(false);
    assert_eq!(size(&mut test.mount, "file.txt"), Ok(5));
    assert_eq!(size(&mut test.mount, "renamed.txt"), Err(Fat32Error::NotFound));

    test.mount
        .fat()
        .rename(&path("file.txt"), &path("renamed.txt"))
        .expect("rename should succeed");

    assert_eq!(size(&mut test.mount, "file.txt"), Err(Fat32Error::NotFound));
    assert_eq!(size(&mut test.mount, "renamed.txt"), Ok(5), "renamed file should be found");
    assert_coherent(&mut test.mount, &["file.txt", "renamed.txt"]);
}

/// Tests that renaming a directory changes the metadata of the paths below it, which the rename
/// does not name.
#[test]
fn stat_follows_directory_rename() {
    let mut test: TestMount = TestMount::new(false);
    test.mount
        .fat()
        .create_new(&path("dir/child.txt"), false, true)
        .expect("create should succeed");
    assert_eq!(size(&mut test.mount, "dir/child.txt"), Ok(0));
    assert_eq!(size(&mut test.mount, "moved/child.txt"), Err(Fat32Error::NotFound));

    test.mount
        .fat()
        .rename(&path("dir"), &path("moved"))
        .expect("rename should succeed");

    assert_eq!(size(&mut test.mount, "dir/child.txt"), Err(Fat32Error::NotFound));
    assert_eq!(size(&mut test.mount, "moved/child.txt"), Ok(0), "moved file should be found");
    assert_coherent(&mut test.mount, &["dir", "dir/child.txt", "moved", "moved/child.txt"]);
}

/// Tests that the metadata of a file follows its removal.
#[test]
fn stat_follows_unlink() {
    let mut test: TestMount = TestMount::new(false);
    assert_eq!(size(&mut test.mount, "file.txt"), Ok(5));

    test.mount
        .fat()
        .unlink(&path("file.txt"))
        .expect("unlink should succeed");

    assert_eq!(size(&mut test.mount, "file.txt"), Err(Fat32Error::NotFound));
    assert_coherent(&mut test.mount, &["file.txt"]);
}

/// Tests that the metadata of a directory follows its creation and its removal.
#[test]
fn stat_follows_directory_changes() {
    let mut test: TestMount = TestMount::new(false);
    assert_eq!(size(&mut test.mount, "made"), Err(Fat32Error::NotFound));
    assert!(test.mount.stat(&path("dir")).expect("directory should exist").is_dir());

    test.mount
        .fat()
        .mkdir(&path("made"))
        .expect("mkdir should succeed");
    test.mount
        .fat()
        .rmdir(&path("dir"))
        .expect("rmdir should succeed");

    assert!(test.mount.stat(&path("made")).expect("created directory should exist").is_dir());
    assert_eq!(size(&mut test.mount, "dir"), Err(Fat32Error::NotFound));
    assert_coherent(&mut test.mount, &["made", "dir"]);
}

/// Tests that the metadata of a file follows the change of its times.
#[test]
fn stat_follows_set_times() {
    let mut test: TestMount = TestMount::new(false);
    let before: Stat = test.mount.stat(&path("file.txt")).expect("file should exist");
    assert_ne!(before.mtime(), FILE_TIME, "file should not already have the time to set");

    test.mount
        .fat()
        .set_times(&path("file.txt"), Some(FILE_TIME), Some(FILE_TIME))
        .expect("set_times should succeed");

    let after: Stat = test.mount.stat(&path("file.txt")).expect("file should exist");
    assert_eq!(after.mtime(), FILE_TIME, "the new modification time should be reported");
    assert_coherent(&mut test.mount, &["file.txt"]);
}

/// Tests that a read-only mount reports the metadata of its filesystem, including when the cache
/// answers.
#[test]
fn readonly_stat_reports_metadata_and_absence() {
    let mut test: TestMount = TestMount::new(true);

    for _ in 0..2 {
        assert_eq!(size(&mut test.mount, "file.txt"), Ok(5));
        assert!(test.mount.stat(&path("dir")).expect("directory should exist").is_dir());
        assert_eq!(size(&mut test.mount, "missing"), Err(Fat32Error::NotFound));
        assert_coherent(&mut test.mount, &["file.txt", "dir", "missing"]);
    }
}
