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

//==================================================================================================
// ListenSocketRequest
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct ListenSocketRequest {
    pub sockfd: i32,
    pub backlog: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(ListenSocketRequest, SystemCallMessage::PAYLOAD_SIZE);

impl ListenSocketRequest {
    pub const PADDING_SIZE: usize =
        SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<i32>() - mem::size_of::<i32>();

    pub fn new(sockfd: i32, backlog: i32) -> Self {
        Self {
            sockfd,
            backlog,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(tid: ThreadIdentifier, sockfd: i32, backlog: i32) -> Message {
        let message: Self = Self::new(sockfd, backlog);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageKind::ListenSocketRequest,
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
// ListenSocketResponse
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct ListenSocketResponse {
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(ListenSocketResponse, SystemCallMessage::PAYLOAD_SIZE);

impl ListenSocketResponse {
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
            SystemCallMessageKind::ListenSocketResponse,
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
