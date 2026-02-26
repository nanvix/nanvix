// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

//! Error types and mapping for FAT filesystem operations.

//==================================================================================================
// Imports
//==================================================================================================

use core::fmt;

use crate::error::FsError;

//==================================================================================================
// Enumerations
//==================================================================================================

/// Error type for memory storage I/O operations.
///
/// This is a minimal error type for `no_std` environments that implements
/// the `fatfs::IoError` trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryIoError {
    /// Attempted to seek beyond the end of the memory region.
    OutOfBounds,
    /// Attempted to seek to a negative position.
    InvalidSeek,
    /// Unexpected end of file (read returned fewer bytes than expected).
    UnexpectedEof,
    /// Write returned zero bytes when more were expected.
    WriteZero,
}

//==================================================================================================
// Trait Implementations
//==================================================================================================

impl fmt::Display for MemoryIoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryIoError::OutOfBounds => {
                write!(f, "seek beyond end of memory region")
            }
            MemoryIoError::InvalidSeek => write!(f, "invalid seek position"),
            MemoryIoError::UnexpectedEof => write!(f, "unexpected end of file"),
            MemoryIoError::WriteZero => write!(f, "write returned zero bytes"),
        }
    }
}

impl ::fatfs::IoError for MemoryIoError {
    fn is_interrupted(&self) -> bool {
        false
    }

    fn new_unexpected_eof_error() -> Self {
        MemoryIoError::UnexpectedEof
    }

    fn new_write_zero_error() -> Self {
        MemoryIoError::WriteZero
    }
}

//==================================================================================================
// Public Functions
//==================================================================================================

/// Maps a `fatfs` error to an [`FsError`].
///
/// # Parameters
///
/// - `err`: The fatfs error to convert.
///
/// # Returns
///
/// The corresponding [`FsError`] variant.
pub fn map_fatfs_error<T>(err: ::fatfs::Error<T>) -> FsError {
    match err {
        ::fatfs::Error::Io(_) => FsError::IoError,
        ::fatfs::Error::UnexpectedEof => FsError::IoError,
        ::fatfs::Error::WriteZero => FsError::IoError,
        ::fatfs::Error::InvalidInput => FsError::InvalidPath,
        ::fatfs::Error::InvalidFileNameLength => FsError::InvalidPath,
        ::fatfs::Error::UnsupportedFileNameCharacter => FsError::InvalidPath,
        ::fatfs::Error::DirectoryIsNotEmpty => FsError::NotEmpty,
        ::fatfs::Error::NotFound => FsError::NotFound,
        ::fatfs::Error::AlreadyExists => FsError::AlreadyExists,
        ::fatfs::Error::CorruptedFileSystem => FsError::IoError,
        ::fatfs::Error::NotEnoughSpace => FsError::NoSpace,
        _ => FsError::IoError,
    }
}
