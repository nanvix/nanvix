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
    SystemCallMessage,
    SystemCallMessageHeader,
};
use ::core::{
    fmt::Debug,
    mem,
};
use ::sys::{
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::sysapi::ffi::c_int;

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
::static_assert::assert_eq_size!(ConnectSocketRequest, SystemCallMessage::PAYLOAD_SIZE);

impl ConnectSocketRequest {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE
        - mem::size_of::<c_int>()
        - mem::size_of::<sockaddr>()
        - mem::size_of::<socklen_t>();

    pub fn new(sockfd: c_int, sockaddr: &sockaddr, socklen: socklen_t) -> Self {
        Self {
            sockfd,
            sockaddr: *sockaddr,
            socklen,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(
        tid: ThreadIdentifier,
        sockfd: c_int,
        sockaddr: &sockaddr,
        socklen: socklen_t,
    ) -> Message {
        let message: Self = Self::new(sockfd, sockaddr, socklen);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::ConnectSocketRequest,
            message.into_bytes(),
        );
        let message: Message = Message::new(
            MessageSender::new(ProcessIdentifier::from(i32::from(tid)), tid),
            MessageReceiver::new(crate::NETWORK_DESTINATION, ThreadIdentifier::NONE),
            MessageType::Ikc,
            None,
            message.into_bytes(),
        );

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
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(ConnectSocketResponse, SystemCallMessage::PAYLOAD_SIZE);

impl ConnectSocketResponse {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE;

    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    pub fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(tid: ThreadIdentifier) -> Message {
        let message: Self = Self::default();
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::ConnectSocketResponse,
            message.into_bytes(),
        );
        let message: Message = Message::new(
            MessageSender::new(crate::NETWORK_SOURCE, ThreadIdentifier::NONE),
            MessageReceiver::new(ProcessIdentifier::from(i32::from(tid)), tid),
            MessageType::Ikc,
            None,
            message.into_bytes(),
        );

        message
    }
}

impl Default for ConnectSocketResponse {
    fn default() -> Self {
        Self {
            _padding: [0; Self::PADDING_SIZE],
        }
    }
}
