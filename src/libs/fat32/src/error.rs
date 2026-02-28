// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

//! Error types for the FAT32 filesystem library.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::fmt;
use ::error::ErrorCode;

//==================================================================================================
// Enumerations
//==================================================================================================

/// Filesystem error codes.
///
/// These map to standard POSIX error codes (via [`ErrorCode`]) for
/// interoperability with C code and the Nanvix kernel.
///
/// # Error Code Mapping
///
/// | Error | [`ErrorCode`] Equivalent |
/// |-------|-------------------------|
/// | NotFound | `NoSuchEntry` (ENOENT) |
/// | NotAFile | `IsDirectory` (EISDIR) |
/// | NotADirectory | `InvalidDirectory` (ENOTDIR) |
/// | InvalidFd | `BadFile` (EBADF) |
/// | InvalidPath | `InvalidArgument` (EINVAL) |
/// | NotInitialized | `InvalidArgument` (EINVAL) |
/// | InvalidSeek | `IllegalSeek` (ESPIPE) |
/// | ReadOnly | `ReadOnlyFileSystem` (EROFS) |
/// | AlreadyExists | `EntryExists` (EEXIST) |
/// | NotEmpty | `DirectoryNotEmpty` (ENOTEMPTY) |
/// | NoSpace | `NoSpaceOnDevice` (ENOSPC) |
/// | TooManyOpenFiles | `TooManyOpenFiles` (EMFILE) |
/// | NotSupported | `OperationNotSupported` (ENOTSUP) |
/// | InvalidArgument | `InvalidArgument` (EINVAL) |
/// | IoError | `IoErr` (EIO) |
/// | OutOfMemory | `OutOfMemory` (ENOMEM) |
/// | FileLocked | `TryAgain` (EAGAIN) |
/// | PermissionDenied | `PermissionDenied` (EACCES) |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fat32Error {
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

impl From<Fat32Error> for ErrorCode {
    fn from(err: Fat32Error) -> Self {
        match err {
            Fat32Error::NotFound => ErrorCode::NoSuchEntry,
            Fat32Error::NotAFile => ErrorCode::IsDirectory,
            Fat32Error::NotADirectory => ErrorCode::InvalidDirectory,
            Fat32Error::InvalidFd => ErrorCode::BadFile,
            Fat32Error::InvalidPath => ErrorCode::InvalidArgument,
            Fat32Error::NotInitialized => ErrorCode::InvalidArgument,
            Fat32Error::InvalidSeek => ErrorCode::IllegalSeek,
            Fat32Error::ReadOnly => ErrorCode::ReadOnlyFileSystem,
            Fat32Error::AlreadyExists => ErrorCode::EntryExists,
            Fat32Error::NotEmpty => ErrorCode::DirectoryNotEmpty,
            Fat32Error::NoSpace => ErrorCode::NoSpaceOnDevice,
            Fat32Error::TooManyOpenFiles => ErrorCode::TooManyOpenFiles,
            Fat32Error::NotSupported => ErrorCode::OperationNotSupported,
            Fat32Error::InvalidArgument => ErrorCode::InvalidArgument,
            Fat32Error::IoError => ErrorCode::IoErr,
            Fat32Error::OutOfMemory => ErrorCode::OutOfMemory,
            Fat32Error::FileLocked => ErrorCode::TryAgain,
            Fat32Error::PermissionDenied => ErrorCode::PermissionDenied,
        }
    }
}

impl fmt::Display for Fat32Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fat32Error::NotFound => write!(f, "file or directory not found"),
            Fat32Error::NotAFile => write!(f, "path is a directory, not a file"),
            Fat32Error::NotADirectory => write!(f, "path is a file, not a directory"),
            Fat32Error::InvalidFd => write!(f, "invalid file descriptor"),
            Fat32Error::InvalidPath => write!(f, "invalid path"),
            Fat32Error::NotInitialized => write!(f, "filesystem not initialized"),
            Fat32Error::InvalidSeek => write!(f, "invalid seek position"),
            Fat32Error::ReadOnly => write!(f, "read-only filesystem"),
            Fat32Error::AlreadyExists => write!(f, "file or directory already exists"),
            Fat32Error::NotEmpty => write!(f, "directory not empty"),
            Fat32Error::NoSpace => write!(f, "no space left on device"),
            Fat32Error::TooManyOpenFiles => write!(f, "too many open files"),
            Fat32Error::NotSupported => write!(f, "operation not supported"),
            Fat32Error::InvalidArgument => write!(f, "invalid argument"),
            Fat32Error::IoError => write!(f, "I/O error"),
            Fat32Error::OutOfMemory => write!(f, "out of memory"),
            Fat32Error::FileLocked => write!(f, "file is locked"),
            Fat32Error::PermissionDenied => write!(f, "permission denied"),
        }
    }
}

impl core::error::Error for Fat32Error {}
