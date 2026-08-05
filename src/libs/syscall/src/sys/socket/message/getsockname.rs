// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    sys::socket::sockaddr,
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
use ::sysapi::ffi::c_int;

//==================================================================================================
// GetSockNameRequest
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct GetSockNameRequest {
    pub sockfd: c_int,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(GetSockNameRequest, SystemCallMessage::PAYLOAD_SIZE);

impl GetSockNameRequest {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<c_int>();

    pub fn new(sockfd: c_int) -> Self {
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

    pub fn build(tid: ThreadIdentifier, sockfd: c_int) -> Message {
        let message: Self = Self::new(sockfd);
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageKind::GetSockNameRequest, message.into_bytes());
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
// GetSockNameResponse
//==================================================================================================

#[repr(C, packed)]
pub struct GetSockNameResponse {
    pub sockaddr: sockaddr,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(GetSockNameResponse, SystemCallMessage::PAYLOAD_SIZE);

impl GetSockNameResponse {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<sockaddr>();

    pub fn new(sockaddr: &sockaddr) -> Self {
        Self {
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

    pub fn build(tid: ThreadIdentifier, sockaddr: &sockaddr) -> Message {
        let message: Self = Self::new(sockaddr);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageKind::GetSockNameResponse,
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
