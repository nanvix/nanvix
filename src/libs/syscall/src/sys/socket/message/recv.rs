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
        SG_BULK_MAX_BYTES,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::sysapi::sys_types::c_size_t;

//==================================================================================================
// ReceiveSocketRequest
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct ReceiveSocketRequest {
    pub sockfd: i32,
    pub count: u32,
    pub flags: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(ReceiveSocketRequest, SystemCallMessage::PAYLOAD_SIZE);

impl ReceiveSocketRequest {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE
        - mem::size_of::<i32>()
        - mem::size_of::<u32>()
        - mem::size_of::<i32>();

    pub fn new(sockfd: i32, count: u32, flags: i32) -> Self {
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

    pub fn build(tid: ThreadIdentifier, sockfd: i32, count: u32, flags: i32) -> Message {
        let message: ReceiveSocketRequest = ReceiveSocketRequest::new(sockfd, count, flags);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageKind::ReceiveSocketRequest,
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
// ReceiveSocketResponse
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct ReceiveSocketResponse {
    pub count: c_size_t,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(ReceiveSocketResponse, SystemCallMessage::PAYLOAD_SIZE);

impl ReceiveSocketResponse {
    /// Maximum number of payload bytes a single `recv()` transfer may carry. The data travels
    /// out-of-band via a scatter/gather pull, so it is bounded by the maximum scatter/gather
    /// transfer size rather than by a single page or by the IPC message payload.
    pub const MAX_DATA_SIZE: usize = SG_BULK_MAX_BYTES;

    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<c_size_t>();

    pub fn new(count: c_size_t) -> Self {
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

    pub fn build(tid: ThreadIdentifier, count: c_size_t) -> Message {
        let message: ReceiveSocketResponse = ReceiveSocketResponse::new(count);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageKind::ReceiveSocketResponse,
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
