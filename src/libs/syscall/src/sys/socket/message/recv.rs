// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::core::mem;
use ::sys::{
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    pm::ThreadIdentifier,
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
::static_assert::assert_eq_size!(ReceiveSocketRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl ReceiveSocketRequest {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE
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

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    pub fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(tid: ThreadIdentifier, sockfd: i32, count: u32, flags: i32) -> Message {
        let message: ReceiveSocketRequest = ReceiveSocketRequest::new(sockfd, count, flags);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::ReceiveSocketRequest,
            message.into_bytes(),
        );
        let message: Message = Message::new(
            MessageSender::from(tid),
            MessageReceiver::from(crate::LINUXD),
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
    pub buffer: [u8; Self::BUFFER_SIZE],
}
::static_assert::assert_eq_size!(ReceiveSocketResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl ReceiveSocketResponse {
    pub const BUFFER_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<c_size_t>();

    pub fn new(count: c_size_t, buffer: [u8; Self::BUFFER_SIZE]) -> Self {
        Self { count, buffer }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    pub fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(
        tid: ThreadIdentifier,
        count: c_size_t,
        buffer: [u8; Self::BUFFER_SIZE],
    ) -> Message {
        let message: ReceiveSocketResponse = ReceiveSocketResponse::new(count, buffer);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::ReceiveSocketResponse,
            message.into_bytes(),
        );
        let message: Message = Message::new(
            MessageSender::from(crate::LINUXD),
            MessageReceiver::from(tid),
            MessageType::Ikc,
            None,
            message.into_bytes(),
        );
        message
    }
}
