// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::c_int,
    sys::socket::sockaddr,
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
// BindSocketRequest
//==================================================================================================

#[repr(C, packed)]
pub struct BindSocketRequest {
    pub sockfd: c_int,
    pub sockaddr: sockaddr,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(BindSocketRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl BindSocketRequest {
    pub const PADDING_SIZE: usize =
        LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<c_int>() - mem::size_of::<sockaddr>();

    pub fn new(sockfd: c_int, sockaddr: &sockaddr) -> Self {
        Self {
            sockfd,
            sockaddr: sockaddr.clone(),
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(pid: ProcessIdentifier, sockfd: c_int, sockaddr: &sockaddr) -> Message {
        let message: Self = Self::new(sockfd, sockaddr);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::BindSocketRequest,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(pid, crate::LINUXD, MessageType::Ikc, None, message.into_bytes());

        message
    }
}

impl Debug for BindSocketRequest {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            "BindSocketRequest {{ sockfd: {}, sockaddr: {:?} }}",
            { self.sockfd },
            &self.sockaddr
        )
    }
}

//==================================================================================================
// BindSocketResponse
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct BindSocketResponse {
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(BindSocketResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl BindSocketResponse {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE;

    fn new() -> Self {
        Self {
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(pid: ProcessIdentifier) -> Message {
        let message: Self = Self::new();
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::BindSocketResponse,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(crate::LINUXD, pid, MessageType::Ikc, None, message.into_bytes());

        message
    }
}
