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
use core::fmt::Debug;

//==================================================================================================
// FileSyncRequest
//==================================================================================================

#[repr(C, packed)]
pub struct FileSyncRequest {
    pub fd: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(FileSyncRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl FileSyncRequest {
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

    pub fn build(pid: ProcessIdentifier, fd: i32) -> Message {
        let message: FileSyncRequest = FileSyncRequest::new(fd);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::FileSyncRequest,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(pid, crate::LINUXD, MessageType::Ikc, None, message.into_bytes());
        message
    }
}

impl Debug for FileSyncRequest {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "FileSyncRequest {{ fd: {} }}", { self.fd })
    }
}

//==================================================================================================
// FileSyncResponse
//==================================================================================================

#[repr(C, packed)]
pub struct FileSyncResponse {
    pub ret: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(FileSyncResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl FileSyncResponse {
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
        let message: FileSyncResponse = FileSyncResponse::new(ret);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::FileSyncResponse,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(crate::LINUXD, pid, MessageType::Ikc, None, message.into_bytes());
        message
    }
}
