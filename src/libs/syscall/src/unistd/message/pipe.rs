// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    SystemCallMessage,
    SystemCallMessageHeader,
};
use ::core::mem;
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

//==================================================================================================
// PipeRequest
//==================================================================================================

#[repr(C, packed)]
pub struct PipeRequest {
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(PipeRequest, SystemCallMessage::PAYLOAD_SIZE);

impl PipeRequest {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE;

    fn new() -> Self {
        Self {
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
        destination: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: PipeRequest = PipeRequest::new();
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageHeader::PipeRequest, message.into_bytes());
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
// PipeResponse
//==================================================================================================

#[repr(C, packed)]
pub struct PipeResponse {
    pub read_fd: i32,
    pub write_fd: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(PipeResponse, SystemCallMessage::PAYLOAD_SIZE);

impl PipeResponse {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE - 2 * mem::size_of::<i32>();

    fn new(read_fd: i32, write_fd: i32) -> Self {
        Self {
            read_fd,
            write_fd,
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
        read_fd: i32,
        write_fd: i32,
        source: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: PipeResponse = PipeResponse::new(read_fd, write_fd);
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageHeader::PipeResponse, message.into_bytes());
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
