// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    fcntl::{
        atflags::AT_FDCWD,
        file_access_mode::O_WRONLY,
        file_creation_flags::{
            O_CREAT,
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
use ::syscall::{
    fcntl,
    unistd,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests that `open()` and `close()` work with paths longer than the old 36-byte inline limit.
///
/// This exercises the VFS open path with long relative paths. The hostfs multi-part
/// messaging path is covered by the mount-test suite (which uses `/mnt`-prefixed paths).
pub fn test() {
    let mode: mode_t = S_IRUSR | S_IWUSR;

    // Create a directory with a long name (exceeds 36-byte inline limit when combined with file).
    let long_dir: &str = "test-open-close-long-path-directory";
    if let Err(e) = ::syscall::sys::stat::mkdir(long_dir, S_IRWXU) {
        panic!("mkdir long dir failed: {e:?}");
    }

    // Full path is 61 bytes — well beyond the old 36-byte MAX_INLINE_PATH_LEN.
    let long_path: &str = "test-open-close-long-path-directory/a-file-with-long-name.txt";

    // Create file with long path.
    let fd: c_int = match fcntl::open(long_path, O_CREAT | O_WRONLY | O_TRUNC, mode) {
        Ok(fd) => fd,
        Err(e) => panic!("open long path failed: {e:?}"),
    };
    if let Err(e) = unistd::close(fd) {
        panic!("close failed: {e:?}");
    }

    // Re-open to verify persistence.
    let fd: c_int = match fcntl::open(long_path, O_WRONLY, 0) {
        Ok(fd) => fd,
        Err(e) => panic!("re-open long path failed: {e:?}"),
    };
    if let Err(e) = unistd::close(fd) {
        panic!("close after re-open failed: {e:?}");
    }

    // Clean up.
    if let Err(e) = unistd::unlink(long_path) {
        panic!("unlink long path failed: {e:?}");
    }
    if let Err(e) = fcntl::unlinkat(AT_FDCWD, long_dir, ::sysapi::fcntl::atflags::AT_REMOVEDIR) {
        panic!("rmdir long dir failed: {e:?}");
    }
}
