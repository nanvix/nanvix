// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Directory listing tests over hostfs: `getdents` (a.k.a. `readdir`).
//!
//! These tests drive the guest-visible directory-iteration API over a `/mnt` hostfs mount
//! and, in doing so, exercise vfsd's asynchronous `getdents` sweep end to end: hostfsd
//! returns one entry per IKC round-trip, so vfsd issues repeated readdir requests under a
//! single pending op and persists a per-FD cursor between `getdents` calls (see
//! `step_getdents`/`finish_getdents` in vfsd). Because vfsd is a `no_std`/`no_main` guest
//! binary that the build system does not host-unit-test, these integration tests are the
//! practical coverage for that sweep logic:
//!
//! - listing completeness and per-entry type reporting (directory vs. regular file);
//! - cursor resumption across many single-entry `readdir` calls (no duplicates or skips);
//! - the multi-entry sweep's "requested count reached" vs. "directory exhausted" exits;
//! - rejection of `getdents` on a non-directory hostfs FD (ENOTDIR).

use ::core::ffi::CStr;
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    dirent::posix_dent,
    fcntl::{
        atflags::{
            AT_FDCWD,
            AT_REMOVEDIR,
        },
        file_access_mode::{
            O_RDONLY,
            O_WRONLY,
        },
        file_creation_flags::{
            O_CREAT,
            O_DIRECTORY,
            O_TRUNC,
        },
    },
    ffi::c_int,
    sys_stat::file_mode::{
        S_IRUSR,
        S_IRWXU,
        S_IWUSR,
    },
};
use ::syscall::safe::{
    dir::{
        closedir,
        opendir,
        readdir,
        RawDirectory,
    },
    FileSystemPath,
    FileType,
};
use alloc::{
    string::{
        String,
        ToString,
    },
    vec::Vec,
};

/// Upper bound on directory-iteration steps before a test gives up.
///
/// A correct sweep terminates once the directory is exhausted; this cap turns a
/// hypothetical non-terminating cursor bug into a clean failure instead of a hang.
const ITERATION_GUARD: usize = 128;

pub fn test() -> Result<(), Error> {
    test_list_and_types()?;
    test_empty_directory()?;
    test_sweep_count_and_eof()?;
    test_getdents_on_regular_file_fails()?;
    Ok(())
}

/// Creates an empty regular file at `path` over hostfs.
fn create_file(path: &str) -> Result<(), Error> {
    let mode: c_int = (S_IRUSR | S_IWUSR) as c_int;
    let fd: c_int =
        ::syscall::fcntl::openat(AT_FDCWD, path, O_CREAT | O_WRONLY | O_TRUNC, mode as u32)?;
    ::syscall::unistd::close(fd)?;
    Ok(())
}

/// Extracts the entry name from a `posix_dent`, or `None` if it is not valid UTF-8.
fn dent_name(dent: &posix_dent) -> Option<String> {
    let cstr: &CStr = CStr::from_bytes_until_nul(&dent.d_name).ok()?;
    ::core::str::from_utf8(cstr.to_bytes())
        .ok()
        .map(ToString::to_string)
}

/// Lists a directory and verifies every created entry is reported exactly once with the
/// correct type.
///
/// The guest `readdir` wrapper requests a single entry per `getdents`, so reading the whole
/// directory drives many cursor-resuming sweeps; a broken cursor would surface here as a
/// duplicated or missing entry (caught by the exact-count check below).
fn test_list_and_types() -> Result<(), Error> {
    let dir: &str = "/mnt/readdir-list";
    ::syscall::sys::stat::mkdir(dir, S_IRWXU)?;
    create_file("/mnt/readdir-list/file-a.txt")?;
    create_file("/mnt/readdir-list/file-b.txt")?;
    ::syscall::sys::stat::mkdir("/mnt/readdir-list/sub-dir", S_IRWXU)?;

    let dirname: FileSystemPath = FileSystemPath::new(dir)?;
    let mut handle: RawDirectory = opendir(&dirname)?;

    let mut names: Vec<String> = Vec::new();
    let mut a_is_file: bool = false;
    let mut b_is_file: bool = false;
    let mut sub_is_dir: bool = false;
    while let Some(entry) = readdir(&mut handle)? {
        let name: String = entry.file_name()?.to_string();
        match name.as_str() {
            "file-a.txt" => a_is_file = entry.file_type() == FileType::RegularFile,
            "file-b.txt" => b_is_file = entry.file_type() == FileType::RegularFile,
            "sub-dir" => sub_is_dir = entry.file_type() == FileType::Directory,
            other => panic!("unexpected entry in {dir}: {other}"),
        }
        names.push(name);
        if names.len() > ITERATION_GUARD {
            panic!("readdir did not terminate for {dir} (possible cursor bug)");
        }
    }
    closedir(&handle)?;

    // Exactly the three created entries — no `.`/`..`, no duplicates, none skipped.
    if names.len() != 3 {
        panic!("expected exactly 3 entries in {dir}, found {}", names.len());
    }
    if !a_is_file {
        panic!("file-a.txt missing or not reported as a regular file");
    }
    if !b_is_file {
        panic!("file-b.txt missing or not reported as a regular file");
    }
    if !sub_is_dir {
        panic!("sub-dir missing or not reported as a directory");
    }
    ::syslog::info!("mount-test: [PASS] readdir lists all entries with correct types");

    // Cleanup.
    ::syscall::fcntl::unlinkat(AT_FDCWD, "/mnt/readdir-list/file-a.txt", 0)?;
    ::syscall::fcntl::unlinkat(AT_FDCWD, "/mnt/readdir-list/file-b.txt", 0)?;
    ::syscall::fcntl::unlinkat(AT_FDCWD, "/mnt/readdir-list/sub-dir", AT_REMOVEDIR)?;
    ::syscall::fcntl::unlinkat(AT_FDCWD, dir, AT_REMOVEDIR)?;
    Ok(())
}

/// Verifies that listing an empty directory yields no entries (immediate end-of-directory).
fn test_empty_directory() -> Result<(), Error> {
    let dir: &str = "/mnt/readdir-empty";
    ::syscall::sys::stat::mkdir(dir, S_IRWXU)?;

    let dirname: FileSystemPath = FileSystemPath::new(dir)?;
    let mut handle: RawDirectory = opendir(&dirname)?;
    let mut count: usize = 0;
    while readdir(&mut handle)?.is_some() {
        count += 1;
        if count > ITERATION_GUARD {
            panic!("empty directory {dir} kept yielding entries (possible end-of-dir bug)");
        }
    }
    closedir(&handle)?;
    if count != 0 {
        panic!("expected 0 entries in empty {dir}, found {count}");
    }
    ::syslog::info!("mount-test: [PASS] readdir on empty directory returns no entries");

    ::syscall::fcntl::unlinkat(AT_FDCWD, dir, AT_REMOVEDIR)?;
    Ok(())
}

/// Drives `getdents` directly with a multi-entry count to exercise both sweep exits.
///
/// With three entries present, a `count == 2` request stops because the requested count is
/// reached, the next request stops because the directory is exhausted before the count is
/// met, and a third request observes immediate end-of-directory. The names gathered across
/// the non-empty requests must equal the created set with no duplicates, proving the per-FD
/// cursor resumes correctly between calls.
fn test_sweep_count_and_eof() -> Result<(), Error> {
    let dir: &str = "/mnt/readdir-sweep";
    ::syscall::sys::stat::mkdir(dir, S_IRWXU)?;
    create_file("/mnt/readdir-sweep/s0")?;
    create_file("/mnt/readdir-sweep/s1")?;
    create_file("/mnt/readdir-sweep/s2")?;

    let fd: c_int = ::syscall::fcntl::openat(AT_FDCWD, dir, O_RDONLY | O_DIRECTORY, 0)?;

    // First sweep: capped at 2 → stops on "requested count reached".
    let first: Vec<posix_dent> = ::syscall::dirent::posix_getdents(fd, 2)?;
    // Second sweep: only one entry left → stops on "directory exhausted" before count.
    let second: Vec<posix_dent> = ::syscall::dirent::posix_getdents(fd, 2)?;
    // Third sweep: cursor already at the end → immediate end-of-directory.
    let third: Vec<posix_dent> = ::syscall::dirent::posix_getdents(fd, 2)?;
    ::syscall::unistd::close(fd)?;

    if first.len() != 2 {
        panic!("expected 2 entries from a count=2 sweep, got {}", first.len());
    }
    if second.len() != 1 {
        panic!("expected 1 trailing entry from the second sweep, got {}", second.len());
    }
    if !third.is_empty() {
        panic!("expected 0 entries past end-of-directory, got {}", third.len());
    }

    // Names across the two non-empty sweeps must be exactly {s0, s1, s2}, each once.
    let mut names: Vec<String> = Vec::new();
    for dent in first.iter().chain(second.iter()) {
        match dent_name(dent) {
            Some(name) => names.push(name),
            None => panic!("getdents returned an entry with an invalid name"),
        }
    }
    for expected in ["s0", "s1", "s2"] {
        let seen: usize = names.iter().filter(|n| n.as_str() == expected).count();
        if seen != 1 {
            panic!("entry {expected} should appear exactly once across sweeps, saw {seen}");
        }
    }
    ::syslog::info!(
        "mount-test: [PASS] getdents count-reached vs end-of-directory with cursor resumption"
    );

    // Cleanup.
    ::syscall::fcntl::unlinkat(AT_FDCWD, "/mnt/readdir-sweep/s0", 0)?;
    ::syscall::fcntl::unlinkat(AT_FDCWD, "/mnt/readdir-sweep/s1", 0)?;
    ::syscall::fcntl::unlinkat(AT_FDCWD, "/mnt/readdir-sweep/s2", 0)?;
    ::syscall::fcntl::unlinkat(AT_FDCWD, dir, AT_REMOVEDIR)?;
    Ok(())
}

/// Verifies that `getdents` on a hostfs *regular file* FD is rejected with ENOTDIR rather
/// than being forwarded to hostfsd (which would masquerade as an empty directory).
fn test_getdents_on_regular_file_fails() -> Result<(), Error> {
    let path: &str = "/mnt/readdir-not-a-dir.txt";
    create_file(path)?;

    let fd: c_int = ::syscall::fcntl::openat(AT_FDCWD, path, O_RDONLY, 0)?;
    let result: Result<Vec<posix_dent>, Error> = ::syscall::dirent::posix_getdents(fd, 4);
    ::syscall::unistd::close(fd)?;
    ::syscall::fcntl::unlinkat(AT_FDCWD, path, 0)?;

    match result {
        Ok(entries) => {
            panic!("getdents on a regular file should fail, got {} entries", entries.len());
        },
        Err(error) => {
            if error.code != ErrorCode::InvalidDirectory {
                panic!("expected InvalidDirectory (ENOTDIR), got {:?}", error.code);
            }
        },
    }
    ::syslog::info!("mount-test: [PASS] getdents on a regular-file FD is rejected with ENOTDIR");
    Ok(())
}
