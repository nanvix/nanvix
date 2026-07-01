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
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::sysapi::{
    sys_socket::socklen_t,
    sys_types::c_size_t,
};

//==================================================================================================
// ReceiveFromSocketRequest
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct ReceiveFromSocketRequest {
    pub sockfd: i32,
    pub count: u32,
    pub flags: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(ReceiveFromSocketRequest, SystemCallMessage::PAYLOAD_SIZE);

impl ReceiveFromSocketRequest {
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
        let message: ReceiveFromSocketRequest = ReceiveFromSocketRequest::new(sockfd, count, flags);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::ReceiveFromSocketRequest,
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
// ReceiveFromSocketResponse
//==================================================================================================

#[repr(C, packed)]
pub struct ReceiveFromSocketResponse {
    pub count: c_size_t,
    pub addrlen: socklen_t,
    pub sockaddr: sockaddr,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(ReceiveFromSocketResponse, SystemCallMessage::PAYLOAD_SIZE);

impl ReceiveFromSocketResponse {
    /// Maximum number of payload bytes a single `recvfrom()` datagram may carry. The data travels
    /// out-of-band via a scatter/gather pull, so it is bounded by a single page rather than by the
    /// IPC message payload.
    pub const MAX_DATA_SIZE: usize = ::arch::mem::PAGE_SIZE;

    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE
        - mem::size_of::<c_size_t>()
        - mem::size_of::<socklen_t>()
        - mem::size_of::<sockaddr>();

    pub fn new(count: c_size_t, addrlen: socklen_t, sockaddr: &sockaddr) -> Self {
        Self {
            count,
            addrlen,
            sockaddr: *sockaddr,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    pub fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(
        tid: ThreadIdentifier,
        count: c_size_t,
        addrlen: socklen_t,
        sockaddr: &sockaddr,
    ) -> Message {
        let message: ReceiveFromSocketResponse =
            ReceiveFromSocketResponse::new(count, addrlen, sockaddr);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::ReceiveFromSocketResponse,
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

impl Debug for ReceiveFromSocketResponse {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        write!(
            f,
            "ReceiveFromSocketResponse {{ count: {}, addrlen: {}, sockaddr: {:?} }}",
            { self.count },
            { self.addrlen },
            self.sockaddr
        )
    }
}
