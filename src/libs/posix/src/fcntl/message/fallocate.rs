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
use ::core::{
    fmt::Debug,
    mem,
};
use ::nvx::{
    ipc::{
        Message,
        MessageType,
    },
    pm::ProcessIdentifier,
    sys::error::Error,
};

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
::nvx::sys::static_assert_size!(FileSpaceControlRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl FileSpaceControlRequest {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE
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

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(
        pid: ProcessIdentifier,
        fd: i32,
        offset: off_t,
        len: off_t,
    ) -> Result<Message, Error> {
        let message: FileSpaceControlRequest = FileSpaceControlRequest::new(fd, offset, len);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::FileSpaceControlRequest,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(pid, crate::LINUXD, MessageType::Ikc, None, message.into_bytes());
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
::nvx::sys::static_assert_size!(FileSpaceControlResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl FileSpaceControlResponse {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    pub fn new(ret: i32) -> Self {
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
        let message: FileSpaceControlResponse = FileSpaceControlResponse::new(ret);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::FileSpaceControlResponse,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(crate::LINUXD, pid, MessageType::Ikc, None, message.into_bytes());
        message
    }
}
