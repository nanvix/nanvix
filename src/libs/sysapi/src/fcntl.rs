// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::{
        c_char,
        c_int,
    },
    sys_types::{
        mode_t,
        off_t,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// File access modes for open(), openat(), and fcntl().
pub mod file_access_mode {
    use crate::ffi::c_int;

    /// Mask for file access mode.
    pub const O_ACCMODE: c_int = 0x3;
    /// Set read-only access.
    pub const O_RDONLY: c_int = 0;
    /// Set write-only access.
    pub const O_WRONLY: c_int = 1;
    /// Set read-write access.
    pub const O_RDWR: c_int = 2;
    /// Open for execute only.
    pub const O_EXEC: c_int = 0x400000;
    /// Open for search only.
    pub const O_SEARCH: c_int = O_EXEC;
}

/// File descriptor flags for use with `fcntl()`.
pub mod file_descriptor_flags {
    use crate::ffi::c_int;

    /// Close-on-exec.
    pub const FD_CLOEXEC: c_int = 1;
    /// Close-on-fork.
    pub const FD_CLOFORK: c_int = 2;
}

/// File creation flags for use in the oflag value to open() and openat().
pub mod file_creation_flags {
    use crate::ffi::c_int;

    /// Create file if it does not exist.
    pub const O_CREAT: c_int = 0x0200;
    /// Truncate file to size zero.
    pub const O_TRUNC: c_int = 0x0400;
    /// Fail if not a new file.
    pub const O_EXCL: c_int = 0x0800;
    /// Do not assign controlling terminal.
    pub const O_NOCTTY: c_int = 0x8000;
    /// Do not follow symbolic links.
    pub const O_NOFOLLOW: c_int = 0x100000;
    /// Fail if path resolves to a non-directory file.
    pub const O_DIRECTORY: c_int = 0x200000;
    /// Close-on-exec flag.
    pub const O_CLOEXEC: c_int = 0x40000;
    /// Close-on-fork flag.
    pub const O_CLOFORK: c_int = 0x80000;

    // TODO: Support O_TTY_INIT
}

/// File status flags for open(), openat(), and fcntl()
pub mod file_status_flags {
    use crate::ffi::c_int;

    /// Set append mode.
    pub const O_APPEND: c_int = 0x0008;
    /// Write I/O operations on the file descriptor will complete as defined by synchronized I/O data integrity completion.
    pub const O_SYNC: c_int = 0x2000;
    /// Non-blocking mode.
    pub const O_NONBLOCK: c_int = 0x4000;
}

/// Constants to be used with `*at()`.
pub mod atflags {
    use crate::ffi::c_int;

    /// Use the current working directory to determine the target of relative file paths.
    pub const AT_FDCWD: c_int = -100;
    /// Check access using effective user and group IDs.
    pub const AT_EACCESS: c_int = 1;
    /// Remove directory instead of file.
    pub const AT_REMOVEDIR: c_int = 8;
    /// Do not follow symbolic links.
    pub const AT_SYMLINK_NOFOLLOW: c_int = 2;
}

/// File descriptor flags for use with `posix_fadvise()`.
pub mod file_advice {
    use crate::ffi::c_int;

    /// The application has no advice to give on its behavior with respect to the specified data
    pub const POSIX_FADV_NORMAL: c_int = 0;
    /// The application expects to access the specified data sequentially from lower offsets to higher offsets.
    pub const POSIX_FADV_SEQUENTIAL: c_int = 1;
    /// The application expects to access the specified data in a random order.
    pub const POSIX_FADV_RANDOM: c_int = 2;
    /// The specified data will be accessed in the near future.
    pub const POSIX_FADV_WILLNEED: c_int = 3;
    /// The specified data will not be accessed in the near future.
    pub const POSIX_FADV_DONTNEED: c_int = 4;
    /// The specified data will be accessed once and then will not be used again.
    pub const POSIX_FADV_NOREUSE: c_int = 5;
}

/// Control requests for use with `fcntl()`.
pub mod file_control_request {
    use crate::ffi::c_int;

    /// Duplicate the file descriptor.
    pub const F_DUPFD: c_int = 0;
    /// Get the file descriptor flags.
    pub const F_GETFD: c_int = 1;
    /// Set the file descriptor flags.
    pub const F_SETFD: c_int = 2;
    /// Get the file status flags and file access modes.
    pub const F_GETFL: c_int = 3;
    /// Set the file status flags.
    pub const F_SETFL: c_int = 4;
    /// Get owner (process or group) of the file.
    pub const F_GETOWN: c_int = 5;
    /// Set owner (process or group) of the file.
    pub const F_SETOWN: c_int = 6;
    // TODO: Support F_GETOWN_EX
    // TODO: Support F_SETOWN_EX
    /// Get record-locking information.
    pub const F_GETLK: c_int = 7;
    /// Set or clear a record-lock (non-blocking).
    pub const F_SETLK: c_int = 8;
    /// Set or clear a record-lock (blocking).
    pub const F_SETLKW: c_int = 9;
    // TODO: Support F_OFD_GETLK
    // TODO: Support F_OFD_SETLK
    // TODO: Support F_OFD_SETLKW
    /// Duplicate the file descriptor and set the close-on-exec flag.
    pub const F_DUPFD_CLOEXEC: c_int = 14;
    /// Duplicate the file descriptor and set the close-on-fork flag.
    pub const F_DUPFD_CLOFORK: c_int = 15;
}

unsafe extern "C" {
    pub fn fcntl(fd: c_int, cmd: c_int, _op: ...);
    pub fn open(path: *const c_char, flags: c_int, mode: mode_t) -> c_int;
    pub fn posix_fallocate(fd: c_int, offset: off_t, len: off_t) -> c_int;
}
