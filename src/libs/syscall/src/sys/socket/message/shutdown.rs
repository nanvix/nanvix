// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    sys::socket::Shutdown,
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

//==================================================================================================
// ShutdownSocketRequest
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct ShutdownSocketRequest {
    pub sockfd: i32,
    pub how: Shutdown,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(ShutdownSocketRequest, SystemCallMessage::PAYLOAD_SIZE);

impl ShutdownSocketRequest {
    pub const PADDING_SIZE: usize =
        SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<i32>() - mem::size_of::<Shutdown>();

    pub fn new(sockfd: i32, how: Shutdown) -> Self {
        Self {
            sockfd,
            how,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(tid: ThreadIdentifier, sockfd: i32, how: Shutdown) -> Message {
        let message: Self = Self::new(sockfd, how);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::ShutdownSocketRequest,
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
// ShutdownSocketResponse
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct ShutdownSocketResponse {
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(ShutdownSocketResponse, SystemCallMessage::PAYLOAD_SIZE);

impl ShutdownSocketResponse {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE;

    fn new() -> Self {
        Self {
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(tid: ThreadIdentifier) -> Message {
        let message: Self = Self::new();
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::ShutdownSocketResponse,
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
