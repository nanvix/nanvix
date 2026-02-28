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

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that `MemoryIoError` variants have non-empty display strings.
    #[test]
    fn memory_io_error_display() {
        let variants: &[MemoryIoError] = &[
            MemoryIoError::OutOfBounds,
            MemoryIoError::InvalidSeek,
            MemoryIoError::UnexpectedEof,
            MemoryIoError::WriteZero,
        ];

        for variant in variants {
            let msg: alloc::string::String = alloc::format!("{variant}");
            assert!(!msg.is_empty(), "display for {variant:?} should not be empty");
        }
    }

    /// Tests that `is_interrupted` always returns false.
    #[test]
    fn memory_io_error_is_not_interrupted() {
        assert!(!::fatfs::IoError::is_interrupted(&MemoryIoError::OutOfBounds));
        assert!(!::fatfs::IoError::is_interrupted(&MemoryIoError::InvalidSeek));
    }

    /// Tests the IoError trait factory methods.
    #[test]
    fn memory_io_error_factory_methods() {
        assert_eq!(
            <MemoryIoError as ::fatfs::IoError>::new_unexpected_eof_error(),
            MemoryIoError::UnexpectedEof,
        );
        assert_eq!(
            <MemoryIoError as ::fatfs::IoError>::new_write_zero_error(),
            MemoryIoError::WriteZero,
        );
    }

    /// Tests mapping of fatfs errors to Fat32Error.
    #[test]
    fn map_fatfs_error_variants() {
        assert_eq!(
            map_fatfs_error::<MemoryIoError>(::fatfs::Error::NotFound),
            Fat32Error::NotFound,
            "NotFound should map to NotFound"
        );
        assert_eq!(
            map_fatfs_error::<MemoryIoError>(::fatfs::Error::AlreadyExists),
            Fat32Error::AlreadyExists,
            "AlreadyExists should map to AlreadyExists"
        );
        assert_eq!(
            map_fatfs_error::<MemoryIoError>(::fatfs::Error::DirectoryIsNotEmpty),
            Fat32Error::NotEmpty,
            "DirectoryIsNotEmpty should map to NotEmpty"
        );
        assert_eq!(
            map_fatfs_error::<MemoryIoError>(::fatfs::Error::NotEnoughSpace),
            Fat32Error::NoSpace,
            "NotEnoughSpace should map to NoSpace"
        );
        assert_eq!(
            map_fatfs_error::<MemoryIoError>(::fatfs::Error::InvalidInput),
            Fat32Error::InvalidPath,
            "InvalidInput should map to InvalidPath"
        );
        assert_eq!(
            map_fatfs_error::<MemoryIoError>(::fatfs::Error::CorruptedFileSystem),
            Fat32Error::IoError,
            "CorruptedFileSystem should map to IoError"
        );
        assert_eq!(
            map_fatfs_error::<MemoryIoError>(::fatfs::Error::UnexpectedEof),
            Fat32Error::IoError,
            "UnexpectedEof should map to IoError"
        );
        assert_eq!(
            map_fatfs_error::<MemoryIoError>(::fatfs::Error::WriteZero),
            Fat32Error::IoError,
            "WriteZero should map to IoError"
        );
    }
}
