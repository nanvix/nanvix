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
use ::syscall::{
    fcntl::{
        openat,
        renameat,
        unlinkat,
    },
    safe::{
        FileSystem,
        FileSystemPath,
        FileSystemPermissions,
        RegularFile,
        RegularFileOpenFlags,
    },
    sys::stat::{
        fstatat,
        mkdir,
    },
    unistd::{
        chdir,
        close,
        getcwd,
        read,
        write,
    },
};
use ::syslog::info;

pub fn test() -> Result<(), Error> {
    test_mkdir_rmdir()?;
    test_create_unlink()?;
    test_rename()?;
    test_unlink_dir_fd()?;
    test_rename_dir_fd()?;
    test_open_close_long_path()?;
    test_routed_directory_paths()?;
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
    let dir = "/mnt/test-renameat-dir";
    let destination_dir = "/mnt/test-renameat-destination";
    let src = "source.txt";
    let dst = "destination.txt";
    let mode = (S_IRUSR | S_IWUSR) as c_int;

    // Create directory and file inside it.
    mkdir(dir, S_IRWXU)?;
    let full_path = "/mnt/test-renameat-dir/source.txt";
    let fd = openat(AT_FDCWD, full_path, O_CREAT | O_WRONLY | O_TRUNC, mode as u32)?;
    close(fd)?;

    // Open directory as dirfd.
    let dirfd = openat(AT_FDCWD, dir, O_RDONLY | O_DIRECTORY, 0)?;

    mkdir(destination_dir, S_IRWXU)?;
    let destination_fd = openat(AT_FDCWD, destination_dir, O_RDONLY | O_DIRECTORY, 0)?;

    // The two operands must retain independent directory anchors through routing.
    renameat(dirfd, src, destination_fd, dst)?;
    info!("mount-test: [PASS] renameat with dirfd");

    // Verify old name is gone, new name exists.
    let result = openat(dirfd, src, O_RDONLY, 0);
    if let Ok(fd) = result {
        // Close the unexpectedly-opened fd before panicking to avoid leaking it
        // (and exhausting the FD table for subsequent tests).
        let _ = close(fd);
        panic!("old file should not exist after renameat with dirfd");
    }
    let new_fd = openat(destination_fd, dst, O_RDONLY, 0)?;
    close(new_fd)?;

    // Clean up.
    unlinkat(destination_fd, dst, 0)?;
    close(destination_fd)?;
    close(dirfd)?;
    unlinkat(AT_FDCWD, destination_dir, AT_REMOVEDIR)?;
    unlinkat(AT_FDCWD, dir, AT_REMOVEDIR)?;

    Ok(())
}

/// Exercises guest path bookkeeping after deferred directory open and chdir completion.
fn test_routed_directory_paths() -> Result<(), Error> {
    use ::sys::error::ErrorCode;
    use ::sysapi::sys_stat::stat;

    const DIRECTORY: &str = "/mnt/routed-dir";
    const NESTED: &str = "/mnt/routed-dir/mnt";
    const SPELLED: &str = "/mnt//routed-dir/./mnt//";
    const FILE: &str = "/mnt/routed-dir/mnt/file";
    const DATA: &[u8] = b"routed-host-file";
    const INVALID_FD: c_int = -99;

    mkdir(DIRECTORY, S_IRWXU)?;
    mkdir(NESTED, S_IRWXU)?;
    let dirfd = openat(INVALID_FD, SPELLED, O_RDONLY | O_DIRECTORY, 0)?;
    let file = openat(dirfd, "./file", O_CREAT | O_WRONLY | O_TRUNC, S_IRUSR | S_IWUSR)?;
    assert_eq!(write(file, DATA)? as usize, DATA.len());

    // Absolute paths ignore both absent and non-directory descriptors.
    for anchor in [INVALID_FD, file] {
        let absolute = openat(anchor, FILE, O_RDONLY, 0)?;
        close(absolute)?;
        let mut st = stat::default();
        fstatat(anchor, FILE, &mut st, 0)?;
        assert_eq!(st.st_size, DATA.len() as i64);
    }
    close(file)?;
    close(dirfd)?;

    // All mount-root spellings send an empty/root payload but must retain a guest path dirfd.
    for root in ["/mnt", "/mnt/", "/mnt//"] {
        let rootfd = openat(AT_FDCWD, root, O_RDONLY | O_DIRECTORY, 0)?;
        let opened = openat(rootfd, "routed-dir/mnt/file", O_RDONLY, 0)?;
        let mut buf = [0; 32];
        let count = read(opened, &mut buf)? as usize;
        assert_eq!(&buf[..count], DATA, "directory completion must retain the guest path prefix");
        close(opened)?;
        close(rootfd)?;
    }

    let previous = getcwd()?;
    chdir(SPELLED)?;
    assert_eq!(getcwd()?.as_str(), NESTED);
    let relative = openat(AT_FDCWD, "./file", O_RDONLY, 0)?;
    close(relative)?;
    assert_eq!(
        chdir("file").err().map(|error| error.code),
        Some(ErrorCode::InvalidDirectory),
        "failed hostfs chdir must not commit a regular file as cwd",
    );
    assert_eq!(getcwd()?.as_str(), NESTED);
    chdir(&previous)?;

    unlinkat(AT_FDCWD, FILE, 0)?;
    unlinkat(AT_FDCWD, NESTED, AT_REMOVEDIR)?;
    unlinkat(AT_FDCWD, DIRECTORY, AT_REMOVEDIR)?;
    info!("mount-test: [PASS] typed hostfs directory paths, root, and cwd routing");
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
