// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    SystemCallMessage,
    SystemCallMessageKind,
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
use ::sysapi::ffi::c_int;

//==================================================================================================
// File Change Directory Request
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct FileChdirRequest {
    /// File descriptor.
    pub fd: c_int,
    /// Padding.
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(FileChdirRequest, SystemCallMessage::PAYLOAD_SIZE);

impl FileChdirRequest {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<c_int>();

    fn new(fd: c_int) -> Self {
        Self {
            fd,
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
        destination: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: FileChdirRequest = FileChdirRequest::new(fd);
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageKind::FileChdirRequest, message.into_bytes());
        let message: Message = Message::new(
            MessageSender::new(ProcessIdentifier::from(i32::from(tid)), tid),
            MessageReceiver::new(destination, ThreadIdentifier::NONE),
            message_type,
            None,
            message.into_bytes(),
        );

        message
    }
}

//==================================================================================================
// File Change Directory Response
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct FileChdirResponse {
    /// Padding.
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(FileChdirResponse, SystemCallMessage::PAYLOAD_SIZE);

impl FileChdirResponse {
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
        source: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: FileChdirResponse = FileChdirResponse::new();
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageKind::FileChdirResponse, message.into_bytes());
        Message::new(
            MessageSender::new(source, ThreadIdentifier::NONE),
            MessageReceiver::new(ProcessIdentifier::from(i32::from(tid)), tid),
            message_type,
            None,
            message.into_bytes(),
        )
    }
}
