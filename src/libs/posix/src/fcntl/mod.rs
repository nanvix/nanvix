// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys::types::mode_t;

//==================================================================================================
// Modules
//==================================================================================================

pub mod message;

cfg_if::cfg_if! {
    if #[cfg(feature = "syscall")] {
        mod syscall;
        pub use self::syscall::{
            openat,
            unlinkat,
            renameat,
            posix_fallocate,
            posix_fadvise,
            symlinkat,
            readlinkat,
            mkdirat,
            fcntl,
            open,
            fchownat,
            fchmodat
        };
    }
}

//==================================================================================================

/// Mask for file access mode.
pub const O_ACCMODE: i32 = 0x3;
/// Set read-only access.
pub const O_RDONLY: i32 = 0;
/// Set write-only access.
pub const O_WRONLY: i32 = 1;
/// Set read-write access.
pub const O_RDWR: i32 = 2;
/// Set append mode.
pub const O_APPEND: i32 = 1 << 5;
/// Create file if it does not exist.
pub const O_CREAT: i32 = 1 << 6;
/// Fail if not a new file.
pub const O_EXCL: i32 = 1 << 7;
/// Truncate file to size zero.
pub const O_TRUNC: i32 = 1 << 8;
/// Remove directory instead of file.
pub const AT_REMOVEDIR: i32 = 1 << 9;
/// Do not follow symbolic links.
pub const AT_SYMLINK_NOFOLLOW: i32 = 1 << 10;
/// Fail if path resolves to a non-directory file.
pub const O_DIRECTORY: i32 = 1 << 11;
/// Write I/O operations on the file descriptor will complete as defined by synchronized I/O file integrity completion.
pub const O_DSYNC: i32 = 1 << 12;
/// Non-blocking mode.
pub const O_NONBLOCK: i32 = 1 << 13;
/// Read I/O operations on the file descriptor shall complete at the same level of integrity as specified by the O_DSYNC and O_SYNC flags
pub const O_RSYNC: i32 = 1 << 14;
/// Write I/O operations on the file descriptor will complete as defined by synchronized I/O data integrity completion.
pub const O_SYNC: i32 = 1 << 15;
/// Open for execute only.
pub const O_EXEC: i32 = 1 << 16;
/// Open for search only.
pub const O_SEARCH: i32 = 1 << 17;

pub const S_IRWXU: mode_t = 0o700;
pub const S_IRUSR: mode_t = 0o400;
pub const S_IWUSR: mode_t = 0o200;
pub const S_IXUSR: mode_t = 0o100;
pub const S_IRWXG: mode_t = 0o070;
pub const S_IRGRP: mode_t = 0o040;
pub const S_IWGRP: mode_t = 0o020;
pub const S_IXGRP: mode_t = 0o010;
pub const S_IRWXO: mode_t = 0o007;
pub const S_IROTH: mode_t = 0o004;
pub const S_IWOTH: mode_t = 0o002;
pub const S_IXOTH: mode_t = 0o001;

/// Use the current working directory to determine the target of relative file paths.
pub const AT_FDCWD: i32 = -100;

/// The application has no advice to give on its behavior with respect to the specified data
pub const POSIX_FADV_NORMAL: i32 = 0;
/// The application expects to access the specified data sequentially from lower offsets to higher offsets.
pub const POSIX_FADV_SEQUENTIAL: i32 = 1;
/// The application expects to access the specified data in a random order.
pub const POSIX_FADV_RANDOM: i32 = 2;
/// The specified data will be accessed in the near future.
pub const POSIX_FADV_WILLNEED: i32 = 3;
/// The specified data will not be accessed in the near future.
pub const POSIX_FADV_DONTNEED: i32 = 4;
/// The specified data will be accessed once and then will not be used again.
pub const POSIX_FADV_NOREUSE: i32 = 5;

/// Duplicate the file descriptor.
pub const F_DUPFD: i32 = 0;
/// Duplicate the file descriptor and set the close-on-exec flag.
pub const F_DUPFD_CLOEXEC: i32 = 1030;
/// Get the file descriptor flags.
pub const F_GETFD: i32 = 1;
/// Set the file descriptor flags.
pub const F_SETFD: i32 = 2;
/// Get the file status flags and file access modes.
pub const F_GETFL: i32 = 3;
/// Set the file status flags.
pub const F_SETFL: i32 = 4;
/// Get owner (process or group) of the file.
pub const F_GETOWN: i32 = 5;
/// Set owner (process or group) of the file.
pub const F_SETOWN: i32 = 6;
// TODO: F_DUPFD_CLOEXEC
// TODO: Support F_GETOWN_EX
// TODO: Support F_SETOWN_EX
// TODO: Support F_GETLK
// TODO: Support F_SETLK
// TODO: Support F_SETLKW
// TODO: Support F_OFD_GETLK
// TODO: Support F_OFD_SETLK
// TODO: Support F_OFD_SETLKW

#[cfg(all(feature = "syscall", feature = "staticlib"))]
pub mod bindings;
