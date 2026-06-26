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
// PartialReadRequest
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct PartialReadRequest {
    pub fd: i32,
    pub count: u32,
    pub offset: off_t,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(PartialReadRequest, SystemCallMessage::PAYLOAD_SIZE);

impl PartialReadRequest {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE
        - mem::size_of::<i32>()
        - mem::size_of::<u32>()
        - mem::size_of::<off_t>();

    fn new(fd: i32, count: c_size_t, offset: off_t) -> Self {
        Self {
            fd,
            count,
            offset,
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
        fd: i32,
        count: c_size_t,
        offset: off_t,
        destination: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: PartialReadRequest = PartialReadRequest::new(fd, count, offset);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::PartialReadRequest,
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
// PartialReadResponse
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct PartialReadResponse {
    pub count: i32,
    pub buffer: [u8; Self::BUFFER_SIZE],
}
::static_assert::assert_eq_size!(PartialReadResponse, SystemCallMessage::PAYLOAD_SIZE);

impl PartialReadResponse {
    pub const BUFFER_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    fn new(count: c_ssize_t, buffer: [u8; Self::BUFFER_SIZE]) -> Self {
        Self { count, buffer }
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
        buffer: [u8; Self::BUFFER_SIZE],
        source: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: PartialReadResponse = PartialReadResponse::new(count, buffer);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::PartialReadResponse,
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
