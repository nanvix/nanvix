// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::c_int,
    sys::types::{
        gid_t,
        uid_t,
    },
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::core::{
    fmt::Debug,
    mem,
};
use ::sys::{
    ipc::{
        Message,
        MessageType,
    },
    pm::ProcessIdentifier,
};

//==================================================================================================
// FileChownRequest
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct FileChownRequest {
    /// File descriptor.
    pub fd: c_int,
    /// Owner.
    pub owner: uid_t,
    /// Group.
    pub group: gid_t,
    /// Padding.
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(FileChownRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl FileChownRequest {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE
        - mem::size_of::<c_int>()
        - mem::size_of::<uid_t>()
        - mem::size_of::<gid_t>();

    fn new(fd: c_int, owner: uid_t, group: gid_t) -> Self {
        Self {
            fd,
            owner,
            group,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(pid: ProcessIdentifier, fd: c_int, owner: uid_t, group: gid_t) -> Message {
        let message: FileChownRequest = FileChownRequest::new(fd, owner, group);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::FileChownRequest,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(pid, crate::LINUXD, MessageType::Ikc, None, message.into_bytes());

        message
    }
}

//==================================================================================================
// FileChownResponse
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct FileChownResponse {
    /// Padding.
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(FileChownResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl FileChownResponse {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE;

    fn new() -> Self {
        Self {
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(pid: ProcessIdentifier) -> Message {
        let message: FileChownResponse = FileChownResponse::new();
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::FileChownResponse,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(crate::LINUXD, pid, MessageType::Ikc, None, message.into_bytes());

        message
    }
}
