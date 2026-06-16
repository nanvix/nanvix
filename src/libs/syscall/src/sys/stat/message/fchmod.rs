// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    SystemCallMessage,
    SystemCallMessageHeader,
};
use ::core::{
    fmt::Debug,
    mem,
};
use ::sys::{
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
use ::sysapi::{
    ffi::c_int,
    sys_types::mode_t,
};

//==================================================================================================
// FileChmodRequest
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct FileChmodRequest {
    /// File descriptor.
    pub fd: c_int,
    /// Mode.
    pub mode: mode_t,
    /// Padding.
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(FileChmodRequest, SystemCallMessage::PAYLOAD_SIZE);

impl FileChmodRequest {
    pub const PADDING_SIZE: usize =
        SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<c_int>() - mem::size_of::<mode_t>();

    fn new(fd: c_int, mode: mode_t) -> Self {
        Self {
            fd,
            mode,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(
        tid: ThreadIdentifier,
        fd: c_int,
        mode: mode_t,
        destination: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: FileChmodRequest = FileChmodRequest::new(fd, mode);
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageHeader::FileChmodRequest, message.into_bytes());
        let message: Message = Message::new(
            MessageSender::from(tid),
            MessageReceiver::from(destination),
            message_type,
            None,
            message.into_bytes(),
        );

        message
    }
}

//==================================================================================================
// FileChmodResponse
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct FileChmodResponse {
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(FileChmodResponse, SystemCallMessage::PAYLOAD_SIZE);

impl FileChmodResponse {
    /// Size of padding.
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE;

    fn new() -> Self {
        Self {
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(
        tid: ThreadIdentifier,
        source: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: FileChmodResponse = FileChmodResponse::new();
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::FileChmodResponse,
            message.into_bytes(),
        );
        let message: Message = Message::new(
            MessageSender::from(source),
            MessageReceiver::from(tid),
            message_type,
            None,
            message.into_bytes(),
        );

        message
    }
}
