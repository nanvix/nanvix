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
use ::sys::{
    ipc::{
        Message,
        MessageType,
    },
    pm::ProcessIdentifier,
};

//==================================================================================================
// ConnectSocketRequest
//==================================================================================================

#[repr(C, packed)]
pub struct ConnectSocketRequest {
    pub sockfd: c_int,
    pub sockaddr: sockaddr,
    pub socklen: socklen_t,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(ConnectSocketRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl ConnectSocketRequest {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE
        - mem::size_of::<c_int>()
        - mem::size_of::<sockaddr>()
        - mem::size_of::<socklen_t>();

    pub fn new(sockfd: c_int, sockaddr: sockaddr, socklen: socklen_t) -> Self {
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
        sockfd: c_int,
        sockaddr: sockaddr,
        socklen: socklen_t,
    ) -> Message {
        let message: Self = Self::new(sockfd, sockaddr, socklen);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::ConnectSocketRequest,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(pid, crate::LINUXD, MessageType::Ikc, None, message.into_bytes());

        message
    }
}

impl Debug for ConnectSocketRequest {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        write!(
            f,
            "ConnectSocketRequest {{ sockfd: {}, sockaddr: {:?}, socklen: {} }}",
            { self.sockfd },
            self.sockaddr,
            { self.socklen }
        )
    }
}

//==================================================================================================
// ConnectSocketResponse
//==================================================================================================

#[repr(C, packed)]
pub struct ConnectSocketResponse {
    pub sockfd: c_int,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(ConnectSocketResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl ConnectSocketResponse {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<c_int>();

    pub fn new(ret: c_int) -> Self {
        Self {
            sockfd: ret,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    pub fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(pid: ProcessIdentifier, ret: c_int) -> Message {
        let message: Self = Self::new(ret);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::ConnectSocketResponse,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(crate::LINUXD, pid, MessageType::Ikc, None, message.into_bytes());

        message
    }
}
