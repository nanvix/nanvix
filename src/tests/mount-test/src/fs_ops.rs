// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Filesystem operation tests over hostfs: mkdir, rmdir, unlink, rename.

use ::sys::error::Error;
use ::sysapi::fcntl::atflags::{
    AT_FDCWD,
    AT_REMOVEDIR,
};
use ::syscall::safe::{
    FileSystem,
    FileSystemPath,
    FileSystemPermissions,
    RegularFile,
    RegularFileOpenFlags,
};

pub fn test() -> Result<(), Error> {
    test_mkdir_rmdir()?;
    test_create_unlink()?;
    test_rename()?;
    Ok(())
}

/// Tests mkdir and rmdir on hostfs.
fn test_mkdir_rmdir() -> Result<(), Error> {
    let dir_path: FileSystemPath = FileSystemPath::new("/mnt/test-dir")?;
    let permissions: FileSystemPermissions = FileSystemPermissions::empty()
        .user_read(true)
        .user_write(true)
        .user_execute(true);

    // Create directory.
    ::syscall::safe::fs::mkdir(&dir_path, permissions)?;
    ::syslog::info!("mount-test: [PASS] mkdir /mnt/test-dir");

    // Create a file inside the new directory.
    {
        let file_path: FileSystemPath = FileSystemPath::new("/mnt/test-dir/inner.txt")?;
        let file_perms: FileSystemPermissions = FileSystemPermissions::empty()
            .user_read(true)
            .user_write(true);
        let mut file: RegularFile = FileSystem::create_regular_file(&file_path, Some(file_perms))?;
        file.write(b"inner-content")?;
        drop(file);

        // Read it back.
        let file: RegularFile =
            FileSystem::open_regular_file(&file_path, &RegularFileOpenFlags::read_only(), None)?;
        let mut buf: [u8; 64] = [0u8; 64];
        let n: usize = file.read(&mut buf)?;
        if &buf[..n] != b"inner-content" {
            panic!("inner.txt content mismatch");
        }
        drop(file);

        // Remove the file before rmdir.
        ::syscall::safe::fs::unlink(&file_path)?;
    }

    // Remove the directory.
    ::syscall::fcntl::unlinkat(AT_FDCWD, "/mnt/test-dir", AT_REMOVEDIR)?;
    ::syslog::info!("mount-test: [PASS] rmdir /mnt/test-dir");

    // Verify that opening the removed directory fails.
    let result = FileSystem::open_regular_file(&dir_path, &RegularFileOpenFlags::read_only(), None);
    if result.is_ok() {
        panic!("open on removed dir should have failed");
    }
    ::syslog::info!("mount-test: [PASS] open removed dir correctly fails");

    Ok(())
}

/// Tests file creation and unlink on hostfs.
fn test_create_unlink() -> Result<(), Error> {
    let pathname: FileSystemPath = FileSystemPath::new("/mnt/test-unlink.txt")?;
    let permissions: FileSystemPermissions = FileSystemPermissions::empty()
        .user_read(true)
        .user_write(true);

    // Create a file.
    {
        let mut file: RegularFile = FileSystem::create_regular_file(&pathname, Some(permissions))?;
        file.write(b"to-be-deleted")?;
    }

    // Unlink the file.
    ::syscall::safe::fs::unlink(&pathname)?;
    ::syslog::info!("mount-test: [PASS] unlink /mnt/test-unlink.txt");

    // Verify the file is gone.
    let result = FileSystem::open_regular_file(&pathname, &RegularFileOpenFlags::read_only(), None);
    if result.is_ok() {
        panic!("open after unlink should have failed");
    }
    ::syslog::info!("mount-test: [PASS] open after unlink correctly fails");

    Ok(())
}

/// Tests rename on hostfs.
fn test_rename() -> Result<(), Error> {
    let old_path: FileSystemPath = FileSystemPath::new("/mnt/ren-src.txt")?;
    let new_path: FileSystemPath = FileSystemPath::new("/mnt/ren-dst.txt")?;
    let permissions: FileSystemPermissions = FileSystemPermissions::empty()
        .user_read(true)
        .user_write(true);

    // Create source file.
    {
        let mut file: RegularFile = FileSystem::create_regular_file(&old_path, Some(permissions))?;
        file.write(b"rename-content")?;
    }

    // Rename.
    ::syscall::safe::fs::rename(&old_path, &new_path)?;
    ::syslog::info!("mount-test: [PASS] rename succeeded");

    // Verify old path is gone.
    let result = FileSystem::open_regular_file(&old_path, &RegularFileOpenFlags::read_only(), None);
    if result.is_ok() {
        panic!("open old path after rename should have failed");
    }

    // Verify new path has correct content.
    {
        let file: RegularFile =
            FileSystem::open_regular_file(&new_path, &RegularFileOpenFlags::read_only(), None)?;
        let mut buf: [u8; 64] = [0u8; 64];
        let n: usize = file.read(&mut buf)?;
        if &buf[..n] != b"rename-content" {
            panic!("renamed file content mismatch");
        }
    }
    ::syslog::info!("mount-test: [PASS] renamed file readable with correct content");

    // Cleanup.
    ::syscall::safe::fs::unlink(&new_path)?;

    Ok(())
}
