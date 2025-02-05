// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::c_int,
    sys::socket::SocketAddr,
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
    pub sockaddr: SocketAddr,
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(BindSocketRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl BindSocketRequest {
    pub const PADDING_SIZE: usize =
        LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<c_int>() - mem::size_of::<SocketAddr>();

    pub fn new(sockfd: c_int, sockaddr: SocketAddr) -> Self {
        Self {
            sockfd,
            sockaddr,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(pid: ProcessIdentifier, sockfd: c_int, sockaddr: SocketAddr) -> Message {
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
        write!(f, "BindSocketRequest {{ sockfd: {}, sockaddr: {:?} }}", { self.sockfd }, {
            self.sockaddr
        },)
    }
}

//==================================================================================================
// BindSocketResponse
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct BindSocketResponse {
    pub ret: c_int,
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(BindSocketResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl BindSocketResponse {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<c_int>();

    pub fn new(sockfd: c_int) -> Self {
        Self {
            ret: sockfd,
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
            LinuxDaemonMessageHeader::BindSocketResponse,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(crate::LINUXD, pid, MessageType::Ikc, None, message.into_bytes());

        message
    }
}
