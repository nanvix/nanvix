// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    SystemCallMessage,
    SystemCallMessageKind,
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
use ::sysapi::sys_socket::sockaddr;

//==================================================================================================
// AcceptSocketRequest
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct AcceptSocketRequest {
    pub sockfd: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(AcceptSocketRequest, SystemCallMessage::PAYLOAD_SIZE);

impl AcceptSocketRequest {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    pub fn new(sockfd: i32) -> Self {
        Self {
            sockfd,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(tid: ThreadIdentifier, sockfd: i32) -> Message {
        let message: Self = Self::new(sockfd);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageKind::AcceptSocketRequest,
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

//==================================================================================================
// AcceptSocketResponse
//==================================================================================================

#[repr(C, packed)]
pub struct AcceptSocketResponse {
    pub sockfd: i32,
    pub sockaddr: sockaddr,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(AcceptSocketResponse, SystemCallMessage::PAYLOAD_SIZE);

impl AcceptSocketResponse {
    pub const PADDING_SIZE: usize =
        SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<i32>() - mem::size_of::<sockaddr>();

    pub fn new(sockfd: i32, sockaddr: &sockaddr) -> Self {
        Self {
            sockfd,
            sockaddr: *sockaddr,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(tid: ThreadIdentifier, sockfd: i32, sockaddr: &sockaddr) -> Message {
        let message: Self = Self::new(sockfd, sockaddr);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageKind::AcceptSocketResponse,
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

impl Debug for AcceptSocketResponse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "AcceptSocketResponse {{ sockfd: {:?}, sockaddr: {:?} }}", { self.sockfd }, {
            {
                self.sockaddr
            }
        })
    }
}
