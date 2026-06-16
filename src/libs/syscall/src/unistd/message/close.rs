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
    fmt,
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

//==================================================================================================
// CloseRequest
//==================================================================================================

#[repr(C, packed)]
pub struct CloseRequest {
    pub fd: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(CloseRequest, SystemCallMessage::PAYLOAD_SIZE);

impl CloseRequest {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    fn new(fd: i32) -> Self {
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
        fd: i32,
        destination: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: CloseRequest = CloseRequest::new(fd);
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageHeader::CloseRequest, message.into_bytes());
        Message::new(
            MessageSender::from(tid),
            MessageReceiver::from(destination),
            message_type,
            None,
            message.into_bytes(),
        )
    }
}

impl fmt::Debug for CloseRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fd: i32 = self.fd;
        write!(f, "{{ fd: {fd} }}")
    }
}

//==================================================================================================
// CloseResponse
//==================================================================================================

#[repr(C, packed)]
pub struct CloseResponse {
    pub ret: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(CloseResponse, SystemCallMessage::PAYLOAD_SIZE);

impl CloseResponse {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    fn new(ret: i32) -> Self {
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
        let message: CloseResponse = CloseResponse::new(ret);
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageHeader::CloseResponse, message.into_bytes());
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
