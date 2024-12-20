// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
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
// AcceptSocketRequest
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct AcceptSocketRequest {
    pub sockfd: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(AcceptSocketRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl AcceptSocketRequest {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    pub fn new(sockfd: i32) -> Self {
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

    pub fn build(pid: ProcessIdentifier, sockfd: i32) -> Message {
        let message: Self = Self::new(sockfd);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::AcceptSocketRequest,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(pid, crate::LINUXD, MessageType::Ikc, None, message.into_bytes());

        message
    }
}

//==================================================================================================
// AcceptSocketResponse
//==================================================================================================

#[repr(C, packed)]
pub struct AcceptSocketResponse {
    pub sockfd: i32,
    pub sockaddr: sockaddr,
    pub socklen: socklen_t,
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(AcceptSocketResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl AcceptSocketResponse {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE
        - mem::size_of::<i32>()
        - mem::size_of::<sockaddr>()
        - mem::size_of::<socklen_t>();

    pub fn new(sockfd: i32, sockaddr: sockaddr, socklen: socklen_t) -> Self {
        Self {
            sockfd,
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

    pub fn build(
        pid: ProcessIdentifier,
        sockfd: i32,
        sockaddr: sockaddr,
        socklen: socklen_t,
    ) -> Message {
        let message: Self = Self::new(sockfd, sockaddr, socklen);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::AcceptSocketResponse,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(crate::LINUXD, pid, MessageType::Ikc, None, message.into_bytes());

        message
    }
}

impl Debug for AcceptSocketResponse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "AcceptSocketResponse {{ sockfd: {}, sockaddr: {:?}, socklen: {} }}",
            { self.sockfd },
            self.sockaddr.clone(),
            { self.socklen }
        )
    }
}
