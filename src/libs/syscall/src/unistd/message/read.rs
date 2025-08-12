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
use ::sysapi::sys_types::c_size_t;

//==================================================================================================
// ReadRequest
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct ReadRequest {
    pub fd: i32,
    pub count: u32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(ReadRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl ReadRequest {
    pub const PADDING_SIZE: usize =
        LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<i32>() - mem::size_of::<u32>();

    fn new(fd: i32, count: c_size_t) -> Self {
        Self {
            fd,
            count,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(tid: ThreadIdentifier, fd: i32, count: c_size_t) -> Message {
        let message: ReadRequest = ReadRequest::new(fd, count);
        let message: LinuxDaemonMessage =
            LinuxDaemonMessage::new(LinuxDaemonMessageHeader::ReadRequest, message.into_bytes());
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
// ReadResponse
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct ReadResponse {
    pub count: i32,
    pub buffer: [u8; Self::BUFFER_SIZE],
}
::static_assert::assert_eq_size!(ReadResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl ReadResponse {
    pub const BUFFER_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    fn new(count: i32, buffer: [u8; Self::BUFFER_SIZE]) -> Self {
        Self { count, buffer }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(tid: ThreadIdentifier, count: i32, buffer: [u8; Self::BUFFER_SIZE]) -> Message {
        let message: ReadResponse = ReadResponse::new(count, buffer);
        let message: LinuxDaemonMessage =
            LinuxDaemonMessage::new(LinuxDaemonMessageHeader::ReadResponse, message.into_bytes());
        let message: Message = Message::new(
            MessageSender::from(crate::LINUXD),
            MessageReceiver::from(tid),
            MessageType::Ikc,
            None,
            message.into_bytes(),
        );
        message
    }

    /// Creates an EOF (end-of-file) ReadResponse with count=0 and an empty buffer.
    pub fn eof(tid: ThreadIdentifier) -> Message {
        Self::build(tid, 0, [0u8; Self::BUFFER_SIZE])
    }
}
