// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    sys::types::off_t,
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
// FileTruncateRequest
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct FileTruncateRequest {
    pub fd: i32,
    pub length: off_t,
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(FileTruncateRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl FileTruncateRequest {
    pub const PADDING_SIZE: usize =
        LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<i32>() - mem::size_of::<off_t>();

    fn new(fd: i32, length: off_t) -> Self {
        Self {
            fd,
            length,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(pid: ProcessIdentifier, fd: i32, length: off_t) -> Message {
        let message: FileTruncateRequest = FileTruncateRequest::new(fd, length);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::FileTruncateRequest,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(pid, crate::LINUXD, MessageType::Ikc, None, message.into_bytes());

        message
    }
}

//==================================================================================================
// FileTruncateResponse
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct FileTruncateResponse {
    pub ret: i32,
    _padding: [u8; Self::PADDING_SIZE],
}

impl FileTruncateResponse {
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

    pub fn build(pid: ProcessIdentifier, ret: i32) -> Message {
        let message: FileTruncateResponse = FileTruncateResponse::new(ret);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::FileTruncateResponse,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(crate::LINUXD, pid, MessageType::Ikc, None, message.into_bytes());

        message
    }
}
