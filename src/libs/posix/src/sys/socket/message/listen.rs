// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
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
};

//==================================================================================================
// ListenSocketRequest
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct ListenSocketRequest {
    pub sockfd: i32,
    pub backlog: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(ListenSocketRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl ListenSocketRequest {
    pub const PADDING_SIZE: usize =
        LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<i32>() - mem::size_of::<i32>();

    pub fn new(sockfd: i32, backlog: i32) -> Self {
        Self {
            sockfd,
            backlog,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(pid: ProcessIdentifier, sockfd: i32, backlog: i32) -> Message {
        let message: Self = Self::new(sockfd, backlog);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::ListenSocketRequest,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(pid, crate::LINUXD, MessageType::Ikc, None, message.into_bytes());

        message
    }
}

//==================================================================================================
// ListenSocketResponse
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct ListenSocketResponse {
    pub ret: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(ListenSocketResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl ListenSocketResponse {
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
        let message: Self = Self::new(ret);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::ListenSocketResponse,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(crate::LINUXD, pid, MessageType::Ikc, None, message.into_bytes());

        message
    }
}
