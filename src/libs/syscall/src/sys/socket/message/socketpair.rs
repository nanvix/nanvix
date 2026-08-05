// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    netinet::in_::Protocol,
    sys::socket::{
        AddressFamily,
        SocketType,
    },
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
// CreateSocketPairRequest
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct CreateSocketPairRequest {
    pub domain: AddressFamily,
    pub typ: SocketType,
    pub protocol: Protocol,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(CreateSocketPairRequest, SystemCallMessage::PAYLOAD_SIZE);

impl CreateSocketPairRequest {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE
        - mem::size_of::<AddressFamily>()
        - mem::size_of::<SocketType>()
        - mem::size_of::<Protocol>();

    pub fn new(domain: AddressFamily, type_: SocketType, protocol: Protocol) -> Self {
        Self {
            domain,
            typ: type_,
            protocol,
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
        domain: AddressFamily,
        typ: SocketType,
        protocol: Protocol,
    ) -> Message {
        let message: Self = Self::new(domain, typ, protocol);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageKind::CreateSocketPairRequest,
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
// CreateSocketPairResponse
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct CreateSocketPairResponse {
    pub sockfd_0: c_int,
    pub sockfd_1: c_int,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(CreateSocketPairResponse, SystemCallMessage::PAYLOAD_SIZE);

impl CreateSocketPairResponse {
    pub const PADDING_SIZE: usize =
        SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<c_int>() - mem::size_of::<c_int>();

    pub fn new(sockfd_0: c_int, sockfd_1: c_int) -> Self {
        Self {
            sockfd_0,
            sockfd_1,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(tid: ThreadIdentifier, sockfd_0: c_int, sockfd_1: c_int) -> Message {
        let message: Self = Self::new(sockfd_0, sockfd_1);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageKind::CreateSocketPairResponse,
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
