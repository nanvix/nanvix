// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::core::mem;
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
// PipeRequest
//==================================================================================================

#[repr(C, packed)]
pub struct PipeRequest {
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(PipeRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl PipeRequest {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE;

    fn new() -> Self {
        Self {
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(tid: ThreadIdentifier) -> Message {
        let message: PipeRequest = PipeRequest::new();
        let message: LinuxDaemonMessage =
            LinuxDaemonMessage::new(LinuxDaemonMessageHeader::PipeRequest, message.into_bytes());
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

//==================================================================================================
// PipeResponse
//==================================================================================================

#[repr(C, packed)]
pub struct PipeResponse {
    pub read_fd: i32,
    pub write_fd: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(PipeResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl PipeResponse {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - 2 * mem::size_of::<i32>();

    fn new(read_fd: i32, write_fd: i32) -> Self {
        Self {
            read_fd,
            write_fd,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(tid: ThreadIdentifier, read_fd: i32, write_fd: i32) -> Message {
        let message: PipeResponse = PipeResponse::new(read_fd, write_fd);
        let message: LinuxDaemonMessage =
            LinuxDaemonMessage::new(LinuxDaemonMessageHeader::PipeResponse, message.into_bytes());
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
