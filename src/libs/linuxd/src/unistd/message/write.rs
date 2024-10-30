// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    sys::types::size_t,
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::core::mem;
use ::nvx::{
    ipc::{
        Message,
        MessageType,
    },
    pm::ProcessIdentifier,
};

//==================================================================================================
// WriteRequest
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct WriteRequest {
    pub fd: i32,
    pub count: size_t,
    pub buffer: [u8; Self::BUFFER_SIZE],
}
::nvx::sys::static_assert_size!(WriteRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl WriteRequest {
    pub const BUFFER_SIZE: usize =
        LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<i32>() - mem::size_of::<size_t>();

    fn new(fd: i32, count: size_t, buffer: [u8; Self::BUFFER_SIZE]) -> Self {
        Self { fd, count, buffer }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(
        pid: ProcessIdentifier,
        fd: i32,
        count: size_t,
        buffer: [u8; Self::BUFFER_SIZE],
    ) -> Message {
        let message: WriteRequest = WriteRequest::new(fd, count, buffer);
        let message: LinuxDaemonMessage =
            LinuxDaemonMessage::new(LinuxDaemonMessageHeader::WriteRequest, message.into_bytes());
        let message: Message =
            Message::new(pid, crate::LINUXD, MessageType::Ikc, None, message.into_bytes());

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
::nvx::sys::static_assert_size!(WriteResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl WriteResponse {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    fn new(count: i32) -> Self {
        Self {
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

    pub fn build(pid: ProcessIdentifier, count: i32) -> Message {
        let message: WriteResponse = WriteResponse::new(count);
        let message: LinuxDaemonMessage =
            LinuxDaemonMessage::new(LinuxDaemonMessageHeader::WriteResponse, message.into_bytes());
        let message: Message =
            Message::new(crate::LINUXD, pid, MessageType::Ikc, None, message.into_bytes());

        message
    }
}
