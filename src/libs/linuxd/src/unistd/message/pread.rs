// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    sys::types::{
        off_t,
        size_t,
        ssize_t,
    },
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
// PartialReadRequest
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct PartialReadRequest {
    pub fd: i32,
    pub count: size_t,
    pub offset: off_t,
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(PartialReadRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl PartialReadRequest {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE
        - mem::size_of::<i32>()
        - mem::size_of::<i32>()
        - mem::size_of::<off_t>();

    fn new(fd: i32, count: size_t, offset: off_t) -> Self {
        Self {
            fd,
            count,
            offset,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(pid: ProcessIdentifier, fd: i32, count: size_t, offset: off_t) -> Message {
        let message: PartialReadRequest = PartialReadRequest::new(fd, count, offset);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::PartialReadRequest,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(pid, crate::LINUXD, MessageType::Ikc, None, message.into_bytes());
        message
    }
}

//==================================================================================================
// PartialReadResponse
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct PartialReadResponse {
    pub count: ssize_t,
    pub buffer: [u8; Self::BUFFER_SIZE],
}
::nvx::sys::static_assert_size!(PartialReadResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl PartialReadResponse {
    pub const BUFFER_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<ssize_t>();

    fn new(count: ssize_t, buffer: [u8; Self::BUFFER_SIZE]) -> Self {
        Self { count, buffer }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(
        pid: ProcessIdentifier,
        count: ssize_t,
        buffer: [u8; Self::BUFFER_SIZE],
    ) -> Message {
        let message: PartialReadResponse = PartialReadResponse::new(count, buffer);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::PartialReadResponse,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(crate::LINUXD, pid, MessageType::Ikc, None, message.into_bytes());
        message
    }
}
