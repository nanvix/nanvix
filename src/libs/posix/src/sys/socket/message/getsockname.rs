// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::c_int,
    sys::socket::{
        sockaddr,
        socklen_t,
    },
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
// GetSockNameRequest
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct GetSockNameRequest {
    pub sockfd: c_int,
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(GetSockNameRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl GetSockNameRequest {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<c_int>();

    pub fn new(sockfd: c_int) -> Self {
        Self {
            sockfd,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(pid: ProcessIdentifier, sockfd: c_int) -> Message {
        let message: Self = Self::new(sockfd);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::GetSockNameRequest,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(pid, crate::LINUXD, MessageType::Ikc, None, message.into_bytes());
        message
    }
}

//==================================================================================================
// GetSockNameResponse
//==================================================================================================

#[repr(C, packed)]
pub struct GetSockNameResponse {
    pub sockaddr: sockaddr,
    pub socklen: socklen_t,
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(GetSockNameResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl GetSockNameResponse {
    pub const PADDING_SIZE: usize =
        LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<sockaddr>() - mem::size_of::<socklen_t>();

    pub fn new(sockaddr: sockaddr, socklen: socklen_t) -> Self {
        Self {
            sockaddr,
            socklen,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(pid: ProcessIdentifier, sockaddr: sockaddr, socklen: socklen_t) -> Message {
        let message: Self = Self::new(sockaddr, socklen);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::GetSockNameResponse,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(crate::LINUXD, pid, MessageType::Ikc, None, message.into_bytes());
        message
    }
}
