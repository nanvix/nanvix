// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    SystemCallMessage,
    SystemCallMessageKind,
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
// SeekRequest
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct SeekRequest {
    pub fd: i32,
    pub offset: i64,
    pub whence: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(SeekRequest, SystemCallMessage::PAYLOAD_SIZE);

impl SeekRequest {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE
        - mem::size_of::<i32>()
        - mem::size_of::<i64>()
        - mem::size_of::<i32>();

    fn new(fd: i32, offset: i64, whence: i32) -> Self {
        Self {
            fd,
            offset,
            whence,
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
        offset: i64,
        whence: i32,
        destination: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: SeekRequest = SeekRequest::new(fd, offset, whence);
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageKind::SeekRequest, message.into_bytes());
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
// SeekResponse
//==================================================================================================

#[repr(C, packed)]
pub struct SeekResponse {
    pub offset: i64,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(SeekResponse, SystemCallMessage::PAYLOAD_SIZE);

impl SeekResponse {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<i64>();

    fn new(offset: i64) -> Self {
        Self {
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
        offset: i64,
        source: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: SeekResponse = SeekResponse::new(offset);
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageKind::SeekResponse, message.into_bytes());
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
