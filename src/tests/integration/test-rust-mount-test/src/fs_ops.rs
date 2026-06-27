// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Filesystem operation tests over hostfs: mkdir, rmdir, unlink, rename.

use ::sys::error::Error;
use ::sysapi::{
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
    test_unlink_dir_fd()?;
    test_rename_dir_fd()?;
    test_open_close_long_path()?;
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

/// Tests `unlinkat()` with a directory file descriptor over hostfs.
fn test_unlink_dir_fd() -> Result<(), Error> {
    let dir: &str = "/mnt/test-unlinkat-dir";
    let file: &str = "dirfd-target.txt";
    let mode: c_int = (S_IRUSR | S_IWUSR) as c_int;

    // Create directory and file inside it.
    ::syscall::sys::stat::mkdir(dir, S_IRWXU)?;
    let full_path: &str = "/mnt/test-unlinkat-dir/dirfd-target.txt";
    let fd: c_int =
        ::syscall::fcntl::openat(AT_FDCWD, full_path, O_CREAT | O_WRONLY | O_TRUNC, mode as u32)?;
    ::syscall::unistd::close(fd)?;

    // Open directory as dirfd.
    let dirfd: c_int = ::syscall::fcntl::openat(AT_FDCWD, dir, O_RDONLY | O_DIRECTORY, 0)?;

    // Unlink using dirfd.
    ::syscall::fcntl::unlinkat(dirfd, file, 0)?;
    ::syslog::info!("mount-test: [PASS] unlinkat with dirfd");

    // Verify file is gone.
    let result = ::syscall::fcntl::openat(dirfd, file, O_RDONLY, 0);
    if let Ok(fd) = result {
        // Close the unexpectedly-opened fd before panicking to avoid leaking it
        // (and exhausting the FD table for subsequent tests).
        let _ = ::syscall::unistd::close(fd);
        panic!("file should not exist after unlinkat with dirfd");
    }

    // Clean up.
    ::syscall::unistd::close(dirfd)?;
    ::syscall::fcntl::unlinkat(AT_FDCWD, dir, AT_REMOVEDIR)?;

    Ok(())
}

/// Tests `renameat()` with directory file descriptors over hostfs.
fn test_rename_dir_fd() -> Result<(), Error> {
    let dir: &str = "/mnt/test-renameat-dir";
    let src: &str = "source.txt";
    let dst: &str = "destination.txt";
    let mode: c_int = (S_IRUSR | S_IWUSR) as c_int;

    // Create directory and file inside it.
    ::syscall::sys::stat::mkdir(dir, S_IRWXU)?;
    let full_path: &str = "/mnt/test-renameat-dir/source.txt";
    let fd: c_int =
        ::syscall::fcntl::openat(AT_FDCWD, full_path, O_CREAT | O_WRONLY | O_TRUNC, mode as u32)?;
    ::syscall::unistd::close(fd)?;

    // Open directory as dirfd.
    let dirfd: c_int = ::syscall::fcntl::openat(AT_FDCWD, dir, O_RDONLY | O_DIRECTORY, 0)?;

    // Rename using dirfd.
    ::syscall::fcntl::renameat(dirfd, src, dirfd, dst)?;
    ::syslog::info!("mount-test: [PASS] renameat with dirfd");

    // Verify old name is gone, new name exists.
    let result = ::syscall::fcntl::openat(dirfd, src, O_RDONLY, 0);
    if let Ok(fd) = result {
        // Close the unexpectedly-opened fd before panicking to avoid leaking it
        // (and exhausting the FD table for subsequent tests).
        let _ = ::syscall::unistd::close(fd);
        panic!("old file should not exist after renameat with dirfd");
    }
    let new_fd: c_int = ::syscall::fcntl::openat(dirfd, dst, O_RDONLY, 0)?;
    ::syscall::unistd::close(new_fd)?;

    // Clean up.
    ::syscall::fcntl::unlinkat(dirfd, dst, 0)?;
    ::syscall::unistd::close(dirfd)?;
    ::syscall::fcntl::unlinkat(AT_FDCWD, dir, AT_REMOVEDIR)?;

    Ok(())
}

/// Tests open/close with paths exceeding the old 36-byte inline message limit over hostfs.
fn test_open_close_long_path() -> Result<(), Error> {
    // Directory + file path totals ~70 bytes — well beyond the old 36-byte inline limit.
    let long_dir: &str = "/mnt/test-open-close-long-path-directory";
    let long_path: &str = "/mnt/test-open-close-long-path-directory/a-file-with-long-name.txt";
    let mode: c_int = (S_IRUSR | S_IWUSR) as c_int;

    // Create directory.
    ::syscall::sys::stat::mkdir(long_dir, S_IRWXU)?;

    // Create file with long path.
    let fd: c_int =
        ::syscall::fcntl::openat(AT_FDCWD, long_path, O_CREAT | O_WRONLY | O_TRUNC, mode as u32)?;
    ::syscall::unistd::close(fd)?;
    ::syslog::info!("mount-test: [PASS] open long path (create)");

    // Re-open to verify persistence.
    let fd: c_int = ::syscall::fcntl::openat(AT_FDCWD, long_path, O_RDONLY, 0)?;
    ::syscall::unistd::close(fd)?;
    ::syslog::info!("mount-test: [PASS] open long path (re-open)");

    // Clean up.
    ::syscall::fcntl::unlinkat(AT_FDCWD, long_path, 0)?;
    ::syscall::fcntl::unlinkat(AT_FDCWD, long_dir, AT_REMOVEDIR)?;

    Ok(())
}
