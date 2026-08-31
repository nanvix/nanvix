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
        RequestIdentifier,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Response Context
//==================================================================================================

/// Routing metadata retained from a request until its response is sent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ResponseContext {
    /// Exact process and thread that issued the request.
    receiver: MessageReceiver,
    /// Identifier that correlates the response with the request.
    request_id: RequestIdentifier,
}

impl ResponseContext {
    /// Creates response routing metadata from a request sender and identifier.
    pub(crate) fn new(sender: MessageSender, request_id: RequestIdentifier) -> Self {
        Self {
            receiver: MessageReceiver::new(sender.pid, sender.tid),
            request_id,
        }
    }

    /// Returns the process that issued the request.
    pub(crate) fn source_pid(self) -> ProcessIdentifier {
        self.receiver.pid
    }

    /// Returns the thread that issued the request.
    pub(crate) fn source_tid(self) -> ThreadIdentifier {
        self.receiver.tid
    }

    /// Returns the request identifier to echo in responses.
    pub(crate) fn request_id(self) -> RequestIdentifier {
        self.request_id
    }

    /// Applies this context to a response message.
    pub(crate) fn prepare_response(self, response: &mut Message) {
        response.destination = self.receiver;
        self.request_id.write_to(response);
    }

    /// Sends a response to the exact requesting thread with the matching request identifier.
    pub(crate) fn send(self, response: &Message) {
        let mut response: Message = response.clone();
        self.prepare_response(&mut response);
        if let Err(e) = ::sys::kcall::ipc::__kcall_send(&response) {
            ::syslog::warn!("send_response(): failed to send response (error={:?})", e);
        }
    }
}

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
        Fat32Error::NoDevice => ErrorCode::NoSuchDeviceOrAddress,
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

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_context_stamps_request_id_and_exact_thread() {
        let process: ProcessIdentifier = ProcessIdentifier::from(10);
        let thread: ThreadIdentifier = ThreadIdentifier::from(20);
        let request_id: RequestIdentifier = RequestIdentifier::from_raw(0x12345678);
        let response_context: ResponseContext =
            ResponseContext::new(MessageSender::new(process, thread), request_id);
        let mut response: Message = build_error(thread, ErrorCode::InvalidMessage);

        response_context.prepare_response(&mut response);

        assert_eq!(
            { response.destination },
            MessageReceiver::new(process, thread),
            "response should target the requesting thread"
        );
        assert_eq!(
            RequestIdentifier::read_from(&response),
            request_id,
            "response should echo the request identifier"
        );
    }
}
