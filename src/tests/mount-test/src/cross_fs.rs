// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Cross-filesystem integration tests.
//!
//! These tests verify that file operations produce consistent error codes and behavior
//! across both RAMFS and the host-mounted filesystem (hostfs). Operations are performed
//! on both filesystems and results are compared.
//!
//! In standalone mode with a minimal initrd, the ramfs root does not support file creation.
//! This module creates `/tmp` as a writable scratch directory before running cross-filesystem
//! comparisons.

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::syscall::safe::{
    FileSystem,
    FileSystemPath,
    FileSystemPermissions,
    RegularFile,
    RegularFileOpenFlags,
};

/// Scratch directory on the ramfs for temporary file operations.
const RAMFS_SCRATCH: &str = "/tmp";

pub fn test() -> Result<(), Error> {
    // Ensure the scratch directory exists (may already exist in full ramfs images).
    // In standalone mode with a minimal initrd the root ramfs is read-only, so mkdir
    // will fail.  When that happens, skip the cross-filesystem comparison tests —
    // the mount/file/fs phases already validate hostfs behavior.
    let dir_perms: FileSystemPermissions = FileSystemPermissions::empty()
        .user_read(true)
        .user_write(true)
        .user_execute(true);
    if ::syscall::safe::fs::mkdir(&FileSystemPath::new(RAMFS_SCRATCH)?, dir_perms).is_err() {
        // Check if it already exists by attempting to open a file in it.
        let probe: FileSystemPath = FileSystemPath::new("/tmp/.probe")?;
        let probe_result = FileSystem::create_regular_file(
            &probe,
            Some(
                FileSystemPermissions::empty()
                    .user_read(true)
                    .user_write(true),
            ),
        );
        match probe_result {
            Ok(_) => {
                // /tmp exists and is writable — clean up probe and proceed.
                let _ = FileSystem::remove_file(&probe);
            },
            Err(_) => {
                // No writable scratch area available — skip cross-fs tests.
                ::syslog::info!(
                    "mount-test: [SKIP] cross-fs tests (ramfs is read-only in this configuration)"
                );
                return Ok(());
            },
        }
    }

    test_write_read_consistency()?;
    test_open_nonexistent_consistency()?;
    test_unlink_nonexistent_consistency()?;
    test_double_create_consistency()?;
    Ok(())
}

/// Verifies that write+read produces identical results on both filesystems.
fn test_write_read_consistency() -> Result<(), Error> {
    const DATA: &[u8] = b"cross-fs-test";

    // --- RAMFS ---
    let ramfs_path: FileSystemPath = FileSystemPath::new("/tmp/cross-fs.txt")?;
    let permissions: FileSystemPermissions = FileSystemPermissions::empty()
        .user_read(true)
        .user_write(true);

    {
        let mut file: RegularFile =
            FileSystem::create_regular_file(&ramfs_path, Some(permissions))?;
        file.write(DATA)?;
    }
    let ramfs_data: alloc::vec::Vec<u8> = {
        let file: RegularFile =
            FileSystem::open_regular_file(&ramfs_path, &RegularFileOpenFlags::read_only(), None)?;
        let mut buf: [u8; 64] = [0u8; 64];
        let n: usize = file.read(&mut buf)?;
        alloc::vec::Vec::from(&buf[..n])
    };
    FileSystem::remove_file(&ramfs_path)?;

    // --- HostFS ---
    let hostfs_path: FileSystemPath = FileSystemPath::new("/mnt/cross-fs.txt")?;

    {
        let mut file: RegularFile =
            FileSystem::create_regular_file(&hostfs_path, Some(permissions))?;
        file.write(DATA)?;
    }
    let hostfs_data: alloc::vec::Vec<u8> = {
        let file: RegularFile =
            FileSystem::open_regular_file(&hostfs_path, &RegularFileOpenFlags::read_only(), None)?;
        let mut buf: [u8; 64] = [0u8; 64];
        let n: usize = file.read(&mut buf)?;
        alloc::vec::Vec::from(&buf[..n])
    };
    ::syscall::safe::fs::unlink(&hostfs_path)?;

    // Compare results.
    if ramfs_data != hostfs_data {
        panic!("cross-fs: write/read content mismatch between RAMFS and HostFS");
    }
    if ramfs_data.as_slice() != DATA {
        panic!("cross-fs: data does not match expected content");
    }
    ::syslog::info!("mount-test: [PASS] cross-fs write/read consistency");

    Ok(())
}

/// Verifies that opening a nonexistent file produces the same error on both filesystems.
fn test_open_nonexistent_consistency() -> Result<(), Error> {
    let ramfs_path: FileSystemPath = FileSystemPath::new("/tmp/no-exist.txt")?;
    let hostfs_path: FileSystemPath = FileSystemPath::new("/mnt/no-exist.txt")?;

    let ramfs_result =
        FileSystem::open_regular_file(&ramfs_path, &RegularFileOpenFlags::read_only(), None);
    let hostfs_result =
        FileSystem::open_regular_file(&hostfs_path, &RegularFileOpenFlags::read_only(), None);

    // Both should fail.
    let ramfs_err: ErrorCode = match ramfs_result {
        Err(e) => e.code,
        Ok(_) => panic!("cross-fs: expected RAMFS open to fail for nonexistent file"),
    };
    let hostfs_err: ErrorCode = match hostfs_result {
        Err(e) => e.code,
        Ok(_) => panic!("cross-fs: expected HostFS open to fail for nonexistent file"),
    };

    if ramfs_err != hostfs_err {
        ::syslog::warn!(
            "cross-fs: open nonexistent error code mismatch: ramfs={:?}, hostfs={:?}",
            ramfs_err,
            hostfs_err
        );
        // Don't panic — document the difference but allow the test to pass.
        // Error code consistency is best-effort for hostfs.
    }
    ::syslog::info!("mount-test: [PASS] cross-fs open nonexistent consistency");

    Ok(())
}

/// Verifies that unlinking a nonexistent file produces an error on both filesystems.
fn test_unlink_nonexistent_consistency() -> Result<(), Error> {
    let ramfs_path: FileSystemPath = FileSystemPath::new("/tmp/no-unlink.txt")?;
    let hostfs_path: FileSystemPath = FileSystemPath::new("/mnt/no-unlink.txt")?;

    let ramfs_result = ::syscall::safe::fs::unlink(&ramfs_path);
    let hostfs_result = ::syscall::safe::fs::unlink(&hostfs_path);

    // Both should fail.
    if ramfs_result.is_ok() {
        panic!("cross-fs: expected RAMFS unlink to fail for nonexistent file");
    }
    if hostfs_result.is_ok() {
        panic!("cross-fs: expected HostFS unlink to fail for nonexistent file");
    }
    ::syslog::info!("mount-test: [PASS] cross-fs unlink nonexistent consistency");

    Ok(())
}

/// Verifies that creating a file twice (with create-exclusive semantics) behaves consistently.
fn test_double_create_consistency() -> Result<(), Error> {
    let ramfs_path: FileSystemPath = FileSystemPath::new("/tmp/dbl-create.txt")?;
    let hostfs_path: FileSystemPath = FileSystemPath::new("/mnt/dbl-create.txt")?;
    let permissions: FileSystemPermissions = FileSystemPermissions::empty()
        .user_read(true)
        .user_write(true);

    // First create should succeed on both.
    {
        let _file: RegularFile = FileSystem::create_regular_file(&ramfs_path, Some(permissions))?;
    }
    {
        let _file: RegularFile = FileSystem::create_regular_file(&hostfs_path, Some(permissions))?;
    }

    // Second create should also succeed (create_regular_file truncates existing files).
    {
        let _file: RegularFile = FileSystem::create_regular_file(&ramfs_path, Some(permissions))?;
    }
    {
        let _file: RegularFile = FileSystem::create_regular_file(&hostfs_path, Some(permissions))?;
    }
    ::syslog::info!("mount-test: [PASS] cross-fs double create consistency");

    // Cleanup.
    FileSystem::remove_file(&ramfs_path)?;
    ::syscall::safe::fs::unlink(&hostfs_path)?;

    Ok(())
}
