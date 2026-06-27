// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    fcntl::{
        atflags::AT_FDCWD,
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
    sys_types::mode_t,
};
use ::syscall::fcntl;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests `renameat()` with directory file descriptors (non-AT_FDCWD).
pub fn test() {
    let dir: &str = "test-renameat-dirfd";
    let src: &str = "src.txt";
    let dst: &str = "dst.txt";
    let mode: mode_t = S_IRUSR | S_IWUSR;

    // Create a test directory.
    if let Err(e) = ::syscall::sys::stat::mkdir(dir, S_IRWXU) {
        panic!("mkdir failed: {e:?}");
    }

    // Create a file inside the directory.
    let full_path: &str = "test-renameat-dirfd/src.txt";
    let fd: c_int = match fcntl::open(full_path, O_CREAT | O_WRONLY | O_TRUNC, mode) {
        Ok(fd) => fd,
        Err(e) => panic!("create file failed: {e:?}"),
    };
    if let Err(e) = ::syscall::unistd::close(fd) {
        panic!("close failed: {e:?}");
    }

    // Open the directory to get a dirfd.
    let dirfd: c_int = match fcntl::openat(AT_FDCWD, dir, O_RDONLY | O_DIRECTORY, 0) {
        Ok(fd) => fd,
        Err(e) => panic!("openat dir failed: {e:?}"),
    };

    // Rename the file using dirfd (not AT_FDCWD).
    if let Err(e) = fcntl::renameat(dirfd, src, dirfd, dst) {
        panic!("renameat with dirfd failed: {e:?}");
    }

    // Verify old name is gone.
    let result = fcntl::openat(dirfd, src, O_RDONLY, 0);
    if let Ok(unexpected_fd) = result {
        // Close the leaked FD to avoid cascading failures from FD-table exhaustion
        // before signaling the test failure.
        let _ = ::syscall::unistd::close(unexpected_fd);
        panic!("old file should not exist after renameat");
    }

    // Verify new name exists.
    let new_fd: c_int = match fcntl::openat(dirfd, dst, O_RDONLY, 0) {
        Ok(fd) => fd,
        Err(e) => panic!("open renamed file failed: {e:?}"),
    };
    if let Err(e) = ::syscall::unistd::close(new_fd) {
        panic!("close new_fd failed: {e:?}");
    }

    // Clean up.
    if let Err(e) = fcntl::unlinkat(dirfd, dst, 0) {
        panic!("unlinkat cleanup failed: {e:?}");
    }
    if let Err(e) = ::syscall::unistd::close(dirfd) {
        panic!("close dirfd failed: {e:?}");
    }
    if let Err(e) = fcntl::unlinkat(AT_FDCWD, dir, ::sysapi::fcntl::atflags::AT_REMOVEDIR) {
        panic!("rmdir cleanup failed: {e:?}");
    }
}
