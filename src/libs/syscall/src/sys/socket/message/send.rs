// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    SystemCallMessage,
    SystemCallMessageKind,
};
use ::core::mem;
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
use ::sysapi::sys_types::{
    c_size_t,
    c_ssize_t,
};

//==================================================================================================
// SendSocketRequest
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct SendSocketRequest {
    pub sockfd: i32,
    pub count: u32,
    pub flags: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(SendSocketRequest, SystemCallMessage::PAYLOAD_SIZE);

impl SendSocketRequest {
    /// Maximum number of payload bytes a single `send()` transfer may carry. The data travels
    /// out-of-band via a scatter/gather push, so it is bounded by a single page rather than by the
    /// IPC message payload. A stream `send()` may transfer fewer bytes than requested, so a caller
    /// with a larger buffer resubmits the remainder on the resulting short send.
    pub const MAX_DATA_SIZE: usize = ::arch::mem::PAGE_SIZE;

    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE
        - mem::size_of::<i32>()
        - mem::size_of::<u32>()
        - mem::size_of::<i32>();

    pub fn new(sockfd: i32, count: c_size_t, flags: i32) -> Self {
        Self {
            sockfd,
            count,
            flags,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    pub fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(tid: ThreadIdentifier, sockfd: i32, count: c_size_t, flags: i32) -> Message {
        let message: SendSocketRequest = SendSocketRequest::new(sockfd, count, flags);
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageKind::SendSocketRequest, message.into_bytes());
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
// SendSocketResponse
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct SendSocketResponse {
    pub count: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(SendSocketResponse, SystemCallMessage::PAYLOAD_SIZE);

impl SendSocketResponse {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    pub fn new(count: c_ssize_t) -> Self {
        Self {
            count,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    pub fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(tid: ThreadIdentifier, count: c_ssize_t) -> Message {
        let message: SendSocketResponse = SendSocketResponse::new(count);
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageKind::SendSocketResponse, message.into_bytes());
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
