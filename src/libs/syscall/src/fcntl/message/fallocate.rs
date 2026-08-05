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
    error::Error,
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
use ::sysapi::sys_types::off_t;

//==================================================================================================
// FileSpaceControlRequest
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct FileSpaceControlRequest {
    pub fd: i32,
    pub offset: off_t,
    pub len: off_t,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(FileSpaceControlRequest, SystemCallMessage::PAYLOAD_SIZE);

impl FileSpaceControlRequest {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE
        - mem::size_of::<i32>()
        - mem::size_of::<off_t>()
        - mem::size_of::<off_t>();

    pub fn new(fd: i32, offset: off_t, len: off_t) -> Self {
        Self {
            fd,
            offset,
            len,
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
        offset: off_t,
        len: off_t,
        destination: ProcessIdentifier,
        message_type: MessageType,
    ) -> Result<Message, Error> {
        let message: FileSpaceControlRequest = FileSpaceControlRequest::new(fd, offset, len);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageKind::FileSpaceControlRequest,
            message.into_bytes(),
        );
        let message: Message = Message::new(
            MessageSender::new(ProcessIdentifier::from(i32::from(tid)), tid),
            MessageReceiver::new(destination, ThreadIdentifier::NONE),
            message_type,
            None,
            message.into_bytes(),
        );
        Ok(message)
    }
}

//==================================================================================================
// FileSpaceControlResponse
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct FileSpaceControlResponse {
    pub ret: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(FileSpaceControlResponse, SystemCallMessage::PAYLOAD_SIZE);

impl FileSpaceControlResponse {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    pub fn new(ret: i32) -> Self {
        Self {
            ret,
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
        ret: i32,
        source: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: FileSpaceControlResponse = FileSpaceControlResponse::new(ret);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageKind::FileSpaceControlResponse,
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
