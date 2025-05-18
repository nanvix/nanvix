// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    sys::types::{
        size_t,
        ssize_t,
    },
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::core::mem;
use ::nvx::{
    ipc::{
        Message,
        MessageType,
    },
    pm::ProcessIdentifier,
};

//==================================================================================================
// SendSocketRequest
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct SendSocketRequest {
    pub sockfd: i32,
    pub count: size_t,
    pub flags: i32,
    pub buffer: [u8; Self::BUFFER_SIZE],
}
::static_assert::assert_eq_size!(SendSocketRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl SendSocketRequest {
    pub const BUFFER_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE
        - mem::size_of::<i32>()
        - mem::size_of::<size_t>()
        - mem::size_of::<i32>();

    pub fn new(sockfd: i32, count: size_t, flags: i32, buffer: [u8; Self::BUFFER_SIZE]) -> Self {
        Self {
            sockfd,
            count,
            flags,
            buffer,
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    pub fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(
        pid: ProcessIdentifier,
        sockfd: i32,
        count: size_t,
        flags: i32,
        buffer: [u8; Self::BUFFER_SIZE],
    ) -> Message {
        let message: SendSocketRequest = SendSocketRequest::new(sockfd, count, flags, buffer);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::SendSocketRequest,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(pid, crate::LINUXD, MessageType::Ikc, None, message.into_bytes());

        message
    }
}

//==================================================================================================
// SendSocketResponse
//==================================================================================================

#[derive(Debug)]
#[repr(C, packed)]
pub struct SendSocketResponse {
    pub count: ssize_t,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(SendSocketResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl SendSocketResponse {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<ssize_t>();

    pub fn new(count: ssize_t) -> Self {
        Self {
            count,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    pub fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(pid: ProcessIdentifier, count: ssize_t) -> Message {
        let message: SendSocketResponse = SendSocketResponse::new(count);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::SendSocketResponse,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(crate::LINUXD, pid, MessageType::Ikc, None, message.into_bytes());

        message
    }
}
