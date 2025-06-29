// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
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
    pm::ThreadIdentifier,
};

//==================================================================================================
// CloseRequest
//==================================================================================================

#[repr(C, packed)]
pub struct CloseRequest {
    pub fd: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(CloseRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl CloseRequest {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    fn new(fd: i32) -> Self {
        Self {
            fd,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(tid: ThreadIdentifier, fd: i32) -> Message {
        let message: CloseRequest = CloseRequest::new(fd);
        let message: LinuxDaemonMessage =
            LinuxDaemonMessage::new(LinuxDaemonMessageHeader::CloseRequest, message.into_bytes());
        let message: Message = Message::new(
            MessageSender::from(tid),
            MessageReceiver::from(crate::LINUXD),
            MessageType::Ikc,
            None,
            message.into_bytes(),
        );

        message
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
::static_assert::assert_eq_size!(CloseResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl CloseResponse {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    fn new(ret: i32) -> Self {
        Self {
            ret,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(tid: ThreadIdentifier, ret: i32) -> Message {
        let message: CloseResponse = CloseResponse::new(ret);
        let message: LinuxDaemonMessage =
            LinuxDaemonMessage::new(LinuxDaemonMessageHeader::CloseResponse, message.into_bytes());
        let message: Message = Message::new(
            MessageSender::from(crate::LINUXD),
            MessageReceiver::from(tid),
            MessageType::Ikc,
            None,
            message.into_bytes(),
        );

        message
    }
}
