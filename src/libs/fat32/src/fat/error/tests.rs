// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

/// Tests that `MemoryIoError` variants have non-empty display strings.
#[test]
fn memory_io_error_display() {
    let variants: &[MemoryIoError] = &[
        MemoryIoError::OutOfBounds,
        MemoryIoError::InvalidSeek,
        MemoryIoError::UnexpectedEof,
        MemoryIoError::WriteZero,
        MemoryIoError::ReadOnly,
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
        map_fatfs_error(::fatfs::Error::NotFound),
        Fat32Error::NotFound,
        "NotFound should map to NotFound"
    );
    assert_eq!(
        map_fatfs_error(::fatfs::Error::AlreadyExists),
        Fat32Error::AlreadyExists,
        "AlreadyExists should map to AlreadyExists"
    );
    assert_eq!(
        map_fatfs_error(::fatfs::Error::DirectoryIsNotEmpty),
        Fat32Error::NotEmpty,
        "DirectoryIsNotEmpty should map to NotEmpty"
    );
    assert_eq!(
        map_fatfs_error(::fatfs::Error::NotEnoughSpace),
        Fat32Error::NoSpace,
        "NotEnoughSpace should map to NoSpace"
    );
    assert_eq!(
        map_fatfs_error(::fatfs::Error::InvalidInput),
        Fat32Error::InvalidPath,
        "InvalidInput should map to InvalidPath"
    );
    assert_eq!(
        map_fatfs_error(::fatfs::Error::CorruptedFileSystem),
        Fat32Error::IoError,
        "CorruptedFileSystem should map to IoError"
    );
    assert_eq!(
        map_fatfs_error(::fatfs::Error::UnexpectedEof),
        Fat32Error::IoError,
        "UnexpectedEof should map to IoError"
    );
    assert_eq!(
        map_fatfs_error(::fatfs::Error::WriteZero),
        Fat32Error::IoError,
        "WriteZero should map to IoError"
    );
    assert_eq!(
        map_fatfs_error(::fatfs::Error::Io(MemoryIoError::ReadOnly)),
        Fat32Error::ReadOnly,
        "Io(ReadOnly) should map to ReadOnly"
    );
}
