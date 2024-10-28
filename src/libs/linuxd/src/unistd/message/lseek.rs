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
use ::nvx::{
    ipc::{
        Message,
        MessageType,
    },
    pm::ProcessIdentifier,
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
::nvx::sys::static_assert_size!(SeekRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl SeekRequest {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE
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

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(pid: ProcessIdentifier, fd: i32, offset: i64, whence: i32) -> Message {
        let message: SeekRequest = SeekRequest::new(fd, offset, whence);
        let message: LinuxDaemonMessage =
            LinuxDaemonMessage::new(LinuxDaemonMessageHeader::SeekRequest, message.into_bytes());
        let message: Message =
            Message::new(pid, crate::LINUXD, MessageType::Ikc, None, message.into_bytes());

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
::nvx::sys::static_assert_size!(SeekResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl SeekResponse {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<i64>();

    fn new(offset: i64) -> Self {
        Self {
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

    pub fn build(pid: ProcessIdentifier, offset: i64) -> Message {
        let message: SeekResponse = SeekResponse::new(offset);
        let message: LinuxDaemonMessage =
            LinuxDaemonMessage::new(LinuxDaemonMessageHeader::SeekResponse, message.into_bytes());
        let message: Message =
            Message::new(crate::LINUXD, pid, MessageType::Ikc, None, message.into_bytes());

        message
    }
}
