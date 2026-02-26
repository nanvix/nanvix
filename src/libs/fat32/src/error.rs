// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

//! Error types for the FAT32 filesystem library.

//==================================================================================================
// Imports
//==================================================================================================

use core::fmt;

//==================================================================================================
// Enumerations
//==================================================================================================

/// Filesystem error codes.
///
/// These map to standard POSIX error codes for interoperability with C code
/// and the nanvix kernel.
///
/// # Error Code Mapping
///
/// | Error | POSIX Equivalent | Code |
/// |-------|-----------------|------|
/// | NotFound | ENOENT | -1 |
/// | NotAFile | EISDIR | -7 |
/// | NotADirectory | ENOTDIR | -6 |
/// | InvalidFd | EBADF | -11 |
/// | InvalidPath | EINVAL | -9 |
/// | NotInitialized | (generic) | -1 |
/// | InvalidSeek | EINVAL | -9 |
/// | ReadOnly | EROFS | -3 |
/// | AlreadyExists | EEXIST | -5 |
/// | NotEmpty | ENOTEMPTY | -8 |
/// | NoSpace | ENOSPC | -4 |
/// | TooManyOpenFiles | EMFILE | -10 |
/// | NotSupported | ENOTSUP | -2 |
/// | InvalidArgument | EINVAL | -9 |
/// | IoError | EIO | -1 |
/// | OutOfMemory | ENOMEM | -1 |
/// | FileLocked | EAGAIN | -1 |
/// | PermissionDenied | EACCES | -12 |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsError {
    /// File or directory not found.
    NotFound,
    /// Path refers to a directory, not a file.
    NotAFile,
    /// Path refers to a file, not a directory.
    NotADirectory,
    /// Invalid file descriptor.
    InvalidFd,
    /// Invalid path (empty, contains null bytes, etc.).
    InvalidPath,
    /// Filesystem not initialized.
    NotInitialized,
    /// Seek to invalid position.
    InvalidSeek,
    /// Path is read-only (cannot write to RO mount).
    ReadOnly,
    /// File or directory already exists.
    AlreadyExists,
    /// Directory is not empty.
    NotEmpty,
    /// No space left on device.
    NoSpace,
    /// Too many open files.
    TooManyOpenFiles,
    /// Operation not supported.
    NotSupported,
    /// Invalid argument.
    InvalidArgument,
    /// I/O error.
    IoError,
    /// Out of memory.
    OutOfMemory,
    /// Resource is in use and cannot be freed (e.g., unmount with open files).
    FileLocked,
    /// Permission denied (cannot perform operation on this resource).
    PermissionDenied,
}

//==================================================================================================
// Trait Implementations
//==================================================================================================

impl FsError {
    /// Converts to a C-style error code.
    ///
    /// # Returns
    ///
    /// A negative integer representing the POSIX error code.
    #[inline]
    pub fn to_c_error(self) -> i32 {
        match self {
            FsError::NotFound => -1,
            FsError::NotAFile => -7,
            FsError::NotADirectory => -6,
            FsError::InvalidFd => -11,
            FsError::InvalidPath => -9,
            FsError::NotInitialized => -1,
            FsError::InvalidSeek => -9,
            FsError::ReadOnly => -3,
            FsError::AlreadyExists => -5,
            FsError::NotEmpty => -8,
            FsError::NoSpace => -4,
            FsError::TooManyOpenFiles => -10,
            FsError::NotSupported => -2,
            FsError::InvalidArgument => -9,
            FsError::IoError => -1,
            FsError::OutOfMemory => -1,
            FsError::FileLocked => -1,
            FsError::PermissionDenied => -12,
        }
    }
}

impl fmt::Display for FsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FsError::NotFound => write!(f, "file or directory not found"),
            FsError::NotAFile => write!(f, "path is a directory, not a file"),
            FsError::NotADirectory => write!(f, "path is a file, not a directory"),
            FsError::InvalidFd => write!(f, "invalid file descriptor"),
            FsError::InvalidPath => write!(f, "invalid path"),
            FsError::NotInitialized => write!(f, "filesystem not initialized"),
            FsError::InvalidSeek => write!(f, "invalid seek position"),
            FsError::ReadOnly => write!(f, "read-only filesystem"),
            FsError::AlreadyExists => write!(f, "file or directory already exists"),
            FsError::NotEmpty => write!(f, "directory not empty"),
            FsError::NoSpace => write!(f, "no space left on device"),
            FsError::TooManyOpenFiles => write!(f, "too many open files"),
            FsError::NotSupported => write!(f, "operation not supported"),
            FsError::InvalidArgument => write!(f, "invalid argument"),
            FsError::IoError => write!(f, "I/O error"),
            FsError::OutOfMemory => write!(f, "out of memory"),
            FsError::FileLocked => write!(f, "file is locked"),
            FsError::PermissionDenied => write!(f, "permission denied"),
        }
    }
}

impl core::error::Error for FsError {}
