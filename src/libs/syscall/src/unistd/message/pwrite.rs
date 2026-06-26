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
    off_t,
};

//==================================================================================================
// PartialWriteRequest
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct PartialWriteRequest {
    pub fd: i32,
    pub count: u32,
    pub offset: off_t,
    pub buffer: [u8; Self::BUFFER_SIZE],
}
::static_assert::assert_eq_size!(PartialWriteRequest, SystemCallMessage::PAYLOAD_SIZE);

impl PartialWriteRequest {
    pub const BUFFER_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE
        - mem::size_of::<i32>()
        - mem::size_of::<u32>()
        - mem::size_of::<off_t>();

    fn new(fd: i32, count: c_size_t, offset: off_t, buffer: [u8; Self::BUFFER_SIZE]) -> Self {
        Self {
            fd,
            count,
            offset,
            buffer,
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
        fd: i32,
        count: c_size_t,
        offset: off_t,
        buffer: [u8; Self::BUFFER_SIZE],
        destination: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: PartialWriteRequest = PartialWriteRequest::new(fd, count, offset, buffer);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::PartialWriteRequest,
            message.into_bytes(),
        );
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
// PartialWriteResponse
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct PartialWriteResponse {
    pub count: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(PartialWriteResponse, SystemCallMessage::PAYLOAD_SIZE);

impl PartialWriteResponse {
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
        let message: PartialWriteResponse = PartialWriteResponse::new(count);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::PartialWriteResponse,
            message.into_bytes(),
        );
        let message: Message = Message::new(
            MessageSender::new(source, ThreadIdentifier::NONE),
            MessageReceiver::new(ProcessIdentifier::from(i32::from(tid)), tid),
            message_type,
            None,
            message.into_bytes(),
        );
        message
    }
}
