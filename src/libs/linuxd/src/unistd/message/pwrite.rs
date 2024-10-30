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
// PartialWriteRequest
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct PartialWriteRequest {
    pub fd: i32,
    pub count: size_t,
    pub offset: off_t,
    pub buffer: [u8; Self::BUFFER_SIZE],
}
::nvx::sys::static_assert_size!(PartialWriteRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl PartialWriteRequest {
    pub const BUFFER_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE
        - mem::size_of::<i32>()
        - mem::size_of::<i32>()
        - mem::size_of::<off_t>();

    fn new(fd: i32, count: size_t, offset: off_t, buffer: [u8; Self::BUFFER_SIZE]) -> Self {
        Self {
            fd,
            count,
            offset,
            buffer,
        }
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
        offset: off_t,
        buffer: [u8; Self::BUFFER_SIZE],
    ) -> Message {
        let message: PartialWriteRequest = PartialWriteRequest::new(fd, count, offset, buffer);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::PartialWriteRequest,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(pid, crate::LINUXD, MessageType::Ikc, None, message.into_bytes());
        message
    }
}

//==================================================================================================
// PartialWriteResponse
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct PartialWriteResponse {
    pub count: ssize_t,
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(PartialWriteResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl PartialWriteResponse {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<ssize_t>();

    fn new(count: ssize_t) -> Self {
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

    pub fn build(pid: ProcessIdentifier, count: ssize_t) -> Message {
        let message: PartialWriteResponse = PartialWriteResponse::new(count);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::PartialWriteResponse,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(crate::LINUXD, pid, MessageType::Ikc, None, message.into_bytes());
        message
    }
}
