// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

/// Tests that every `Fat32Error` variant maps to the expected `ErrorCode`.
#[test]
fn error_code_mapping_is_exhaustive() {
    let cases: &[(Fat32Error, ErrorCode)] = &[
        (Fat32Error::NotFound, ErrorCode::NoSuchEntry),
        (Fat32Error::NotAFile, ErrorCode::IsDirectory),
        (Fat32Error::NotADirectory, ErrorCode::InvalidDirectory),
        (Fat32Error::InvalidFd, ErrorCode::BadFile),
        (Fat32Error::InvalidPath, ErrorCode::InvalidArgument),
        (Fat32Error::NotInitialized, ErrorCode::InvalidArgument),
        (Fat32Error::InvalidSeek, ErrorCode::IllegalSeek),
        (Fat32Error::ReadOnly, ErrorCode::ReadOnlyFileSystem),
        (Fat32Error::AlreadyExists, ErrorCode::EntryExists),
        (Fat32Error::NotEmpty, ErrorCode::DirectoryNotEmpty),
        (Fat32Error::NoSpace, ErrorCode::NoSpaceOnDevice),
        (Fat32Error::TooManyOpenFiles, ErrorCode::TooManyOpenFiles),
        (Fat32Error::NotSupported, ErrorCode::OperationNotSupported),
        (Fat32Error::InvalidArgument, ErrorCode::InvalidArgument),
        (Fat32Error::IoError, ErrorCode::IoErr),
        (Fat32Error::OutOfMemory, ErrorCode::OutOfMemory),
        (Fat32Error::FileLocked, ErrorCode::TryAgain),
        (Fat32Error::NoDevice, ErrorCode::NoSuchDeviceOrAddress),
        (Fat32Error::PermissionDenied, ErrorCode::PermissionDenied),
    ];

    for (fat_err, expected_code) in cases {
        let code: ErrorCode = ErrorCode::from(*fat_err);
        assert_eq!(code, *expected_code, "mapping for {fat_err:?}");
    }
}

/// Tests that every variant has a non-empty display string.
#[test]
fn display_strings_are_non_empty() {
    let variants: &[Fat32Error] = &[
        Fat32Error::NotFound,
        Fat32Error::NotAFile,
        Fat32Error::NotADirectory,
        Fat32Error::InvalidFd,
        Fat32Error::InvalidPath,
        Fat32Error::NotInitialized,
        Fat32Error::InvalidSeek,
        Fat32Error::ReadOnly,
        Fat32Error::AlreadyExists,
        Fat32Error::NotEmpty,
        Fat32Error::NoSpace,
        Fat32Error::TooManyOpenFiles,
        Fat32Error::NotSupported,
        Fat32Error::InvalidArgument,
        Fat32Error::IoError,
        Fat32Error::OutOfMemory,
        Fat32Error::FileLocked,
        Fat32Error::NoDevice,
        Fat32Error::PermissionDenied,
    ];

    for variant in variants {
        let msg: alloc::string::String = alloc::format!("{variant}");
        assert!(!msg.is_empty(), "display for {variant:?} should not be empty");
    }
}

/// Tests that `Fat32Error` implements `Clone` and `PartialEq` correctly.
#[test]
fn clone_and_eq() {
    let err: Fat32Error = Fat32Error::NotFound;
    let cloned: Fat32Error = err;
    assert_eq!(err, cloned, "clone should preserve equality");
    assert_ne!(err, Fat32Error::IoError, "different variants should not be equal");
}
