// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::{
    error::ErrorCode,
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Helper: Fat32Error → ErrorCode
//==================================================================================================

pub(crate) fn fat32_to_error_code(e: &::vfs::Fat32Error) -> ErrorCode {
    use ::vfs::Fat32Error;
    match e {
        Fat32Error::NotFound => ErrorCode::NoSuchEntry,
        Fat32Error::NotAFile => ErrorCode::IsDirectory,
        Fat32Error::NotADirectory => ErrorCode::InvalidDirectory,
        Fat32Error::InvalidFd => ErrorCode::BadFile,
        Fat32Error::InvalidPath => ErrorCode::InvalidArgument,
        Fat32Error::NotInitialized => ErrorCode::InvalidArgument,
        Fat32Error::InvalidSeek => ErrorCode::InvalidArgument,
        Fat32Error::ReadOnly => ErrorCode::PermissionDenied,
        Fat32Error::AlreadyExists => ErrorCode::EntryExists,
        Fat32Error::NotEmpty => ErrorCode::DirectoryNotEmpty,
        Fat32Error::NoSpace => ErrorCode::NoSpaceOnDevice,
        Fat32Error::TooManyOpenFiles => ErrorCode::TooManyOpenFiles,
        Fat32Error::NotSupported => ErrorCode::OperationNotSupported,
        Fat32Error::InvalidArgument => ErrorCode::InvalidArgument,
        Fat32Error::IoError => ErrorCode::IoErr,
        Fat32Error::OutOfMemory => ErrorCode::OutOfMemory,
        Fat32Error::FileLocked => ErrorCode::ResourceBusy,
        Fat32Error::PermissionDenied => ErrorCode::PermissionDenied,
    }
}

//==================================================================================================
// Helper: Build error response
//==================================================================================================

pub(crate) fn build_error(source: ThreadIdentifier, code: ErrorCode) -> Message {
    Message::new(
        MessageSender::VFSD,
        MessageReceiver::new(ProcessIdentifier::from(i32::from(source)), source),
        MessageType::Ipc,
        Some(code),
        [0u8; Message::PAYLOAD_SIZE],
    )
}

/// Sends a response message, logging a warning on failure.
pub(crate) fn send_response(response: &Message) {
    if let Err(e) = ::sys::kcall::ipc::__kcall_send(response) {
        ::syslog::warn!("send_response(): failed to send response (error={:?})", e);
    }
}
