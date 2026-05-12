// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    sys::socket::sockaddr,
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
    pm::ThreadIdentifier,
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
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::GetSockNameRequest,
            message.into_bytes(),
        );
        let message: Message = Message::new(
            MessageSender::from(tid),
            MessageReceiver::from(crate::NETWORK_DESTINATION),
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
            SystemCallMessageHeader::GetSockNameResponse,
            message.into_bytes(),
        );
        let message: Message = Message::new(
            MessageSender::from(crate::NETWORK_SOURCE),
            MessageReceiver::from(tid),
            MessageType::Ikc,
            None,
            message.into_bytes(),
        );
        message
    }
}
