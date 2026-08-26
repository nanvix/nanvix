// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Host filesystem error codes.

/// Error code: operation not permitted.
pub const HOSTFS_ERR_NOT_PERMITTED: i32 = -1;
/// Error code: file or directory not found.
pub const HOSTFS_ERR_NOT_FOUND: i32 = -2;
/// Error code: generic I/O error.
pub const HOSTFS_ERR_IO: i32 = -5;
/// Error code: permission denied.
pub const HOSTFS_ERR_PERMISSION: i32 = -13;
/// Error code: file or directory already exists.
pub const HOSTFS_ERR_EXISTS: i32 = -17;
/// Error code: not a directory.
pub const HOSTFS_ERR_NOT_DIR: i32 = -20;
/// Error code: is a directory.
pub const HOSTFS_ERR_IS_DIR: i32 = -21;
/// Error code: invalid argument.
pub const HOSTFS_ERR_INVALID: i32 = -22;
/// Error code: too many levels of symbolic links (ELOOP).
pub const HOSTFS_ERR_LOOP: i32 = -40;
/// Error code: directory not empty.
pub const HOSTFS_ERR_NOT_EMPTY: i32 = -90;
/// Error code: operation not supported on this host platform (EOPNOTSUPP).
///
/// Currently used by the symlink operation on Windows when the process lacks the
/// privilege to create symbolic links (no Developer Mode and no
/// `SeCreateSymbolicLinkPrivilege`).
pub const HOSTFS_ERR_NOT_SUPPORTED: i32 = -95;
