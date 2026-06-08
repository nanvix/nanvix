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
use ::sysapi::sys_types::{
    c_size_t,
    c_ssize_t,
};

//==================================================================================================
// WriteRequest
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct WriteRequest {
    pub fd: i32,
    pub count: u32,
    pub buffer: [u8; Self::BUFFER_SIZE],
}
::static_assert::assert_eq_size!(WriteRequest, SystemCallMessage::PAYLOAD_SIZE);

impl WriteRequest {
    pub const BUFFER_SIZE: usize =
        SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<i32>() - mem::size_of::<u32>();

    fn new(fd: i32, count: c_size_t, buffer: [u8; Self::BUFFER_SIZE]) -> Self {
        Self { fd, count, buffer }
    }

    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(
        tid: ThreadIdentifier,
        fd: i32,
        count: c_size_t,
        buffer: [u8; Self::BUFFER_SIZE],
        destination: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: WriteRequest = WriteRequest::new(fd, count, buffer);
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageHeader::WriteRequest, message.into_bytes());
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
// WriteResponse
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct WriteResponse {
    pub count: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(WriteResponse, SystemCallMessage::PAYLOAD_SIZE);

impl WriteResponse {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    fn new(count: c_ssize_t) -> Self {
        Self {
            count,
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
        count: c_ssize_t,
        source: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: WriteResponse = WriteResponse::new(count);
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageHeader::WriteResponse, message.into_bytes());
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
