// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

//! Error types and mapping for FAT filesystem operations.

//==================================================================================================
// Imports
//==================================================================================================

use crate::error::Fat32Error;
use ::core::fmt;

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
            },
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

/// Maps a `fatfs` error to an [`Fat32Error`].
///
/// # Parameters
///
/// - `err`: The fatfs error to convert.
///
/// # Returns
///
/// The corresponding [`Fat32Error`] variant.
pub fn map_fatfs_error<T>(err: ::fatfs::Error<T>) -> Fat32Error {
    match err {
        ::fatfs::Error::Io(_) => Fat32Error::IoError,
        ::fatfs::Error::UnexpectedEof => Fat32Error::IoError,
        ::fatfs::Error::WriteZero => Fat32Error::IoError,
        ::fatfs::Error::InvalidInput => Fat32Error::InvalidPath,
        ::fatfs::Error::InvalidFileNameLength => Fat32Error::InvalidPath,
        ::fatfs::Error::UnsupportedFileNameCharacter => Fat32Error::InvalidPath,
        ::fatfs::Error::DirectoryIsNotEmpty => Fat32Error::NotEmpty,
        ::fatfs::Error::NotFound => Fat32Error::NotFound,
        ::fatfs::Error::AlreadyExists => Fat32Error::AlreadyExists,
        ::fatfs::Error::CorruptedFileSystem => Fat32Error::IoError,
        ::fatfs::Error::NotEnoughSpace => Fat32Error::NoSpace,
        _ => Fat32Error::IoError,
    }
}
