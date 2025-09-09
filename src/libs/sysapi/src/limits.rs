// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys_types::c_ssize_t;

//==================================================================================================
// Constants
//==================================================================================================

/// The number of data keys per process.
pub const _POSIX_THREAD_KEYS_MAX: usize = 128;

/// Maximum number of [`crate::sys::uio::iovec`] structures that can be passed to a single call to
/// [`crate::sys::uio::writev`] or [`crate::sys::uio::readv`].
pub const IOV_MAX: usize = 16;

// Maximum length of a host name (not including the terminating null byte).
pub const HOST_NAME_MAX: usize = POSIX_HOST_NAME_MAX;

/// Maximum number of bytes in a pathname (not including the terminating null byte).
pub const POSIX_HOST_NAME_MAX: usize = 255;

/// Maximum number of bytes in a filename (not including the terminating null of a filename string).
pub const NAME_MAX: usize = XOPEN_NAME_MAX;

/// Maximum number of bytes in a filename (not including the terminating null of a filename string).
pub const POSIX_NAME_MAX: usize = 14;

/// Maximum number of bytes in a filename (not including the terminating null of a filename string).
pub const XOPEN_NAME_MAX: usize = 255;

/// Maximum number of files that can be opened by a process.
pub const OPEN_MAX: usize = 64;

/// POSIX-mandated minimum number of files that a process must be able to open.
pub const POSIX_OPEN_MAX: usize = 20;

/// Maximum number of bytes the implementation stores as a pathname in a user-supplied buffer of
/// unspecified size, including the terminating null character. Minimum number the implementation
/// shall accept as the maximum number of bytes in a pathname.
pub const PATH_MAX: usize = XOPEN_PATH_MAX;

/// Maximum number of bytes the implementation stores as a pathname in a user-supplied buffer of
/// unspecified size, including the terminating null character. Minimum number the implementation
/// shall accept as the maximum number of bytes in a pathname.
pub const POSIX_PATH_MAX: usize = 256;

/// Maximum number of bytes the implementation stores as a pathname in a user-supplied buffer of
/// unspecified size, including the terminating null character. Minimum number the implementation
/// shall accept as the maximum number of bytes in a pathname.
pub const XOPEN_PATH_MAX: usize = 1024;

/// Maximum value for an object of type [`crate::sys::types::ssize_t`].
pub const SSIZE_MAX: c_ssize_t = c_ssize_t::MAX;

/// Maximum number of data keys that can be created by a process.
pub const PTHREAD_KEYS_MAX: usize = _POSIX_THREAD_KEYS_MAX;
