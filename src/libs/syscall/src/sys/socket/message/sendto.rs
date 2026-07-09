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
        SG_BULK_MAX_BYTES,
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
// SendToSocketRequest
//==================================================================================================

#[repr(C, packed)]
pub struct SendToSocketRequest {
    pub sockfd: i32,
    pub count: u32,
    pub flags: i32,
    pub sockaddr: sockaddr,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(SendToSocketRequest, SystemCallMessage::PAYLOAD_SIZE);

impl SendToSocketRequest {
    /// Maximum number of payload bytes a single `sendto()` datagram may carry. The data travels
    /// out-of-band via a scatter/gather push, so it is bounded by the maximum scatter/gather
    /// transfer size rather than by a single page or by the IPC message payload.
    pub const MAX_DATA_SIZE: usize = SG_BULK_MAX_BYTES;

    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE
        - mem::size_of::<i32>()
        - mem::size_of::<u32>()
        - mem::size_of::<i32>()
        - mem::size_of::<sockaddr>();

    pub fn new(sockfd: i32, count: c_size_t, flags: i32, sockaddr: &sockaddr) -> Self {
        Self {
            sockfd,
            count,
            flags,
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
        sockfd: i32,
        count: c_size_t,
        flags: i32,
        sockaddr: &sockaddr,
    ) -> Message {
        let message: SendToSocketRequest = SendToSocketRequest::new(sockfd, count, flags, sockaddr);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::SendToSocketRequest,
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

impl Debug for SendToSocketRequest {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        write!(
            f,
            "SendToSocketRequest {{ sockfd: {}, count: {}, flags: {}, sockaddr: {:?} }}",
            { self.sockfd },
            { self.count },
            { self.flags },
            self.sockaddr
        )
    }
}

//==================================================================================================
// SendToSocketResponse
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct SendToSocketResponse {
    pub count: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(SendToSocketResponse, SystemCallMessage::PAYLOAD_SIZE);

impl SendToSocketResponse {
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
        let message: SendToSocketResponse = SendToSocketResponse::new(count);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::SendToSocketResponse,
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
