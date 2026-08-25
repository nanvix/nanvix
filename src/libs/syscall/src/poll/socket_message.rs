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
use ::sysapi::ffi::c_short;

//==================================================================================================
// Structures
//==================================================================================================

/// Request for a non-blocking socket readiness snapshot.
#[derive(Debug)]
#[repr(C, packed)]
pub struct PollSocketRequest {
    sockfd: i32,
    events: c_short,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(PollSocketRequest, SystemCallMessage::PAYLOAD_SIZE);

impl PollSocketRequest {
    const PADDING_SIZE: usize =
        SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<i32>() - mem::size_of::<c_short>();

    /// Creates a socket poll request.
    pub fn new(sockfd: i32, events: c_short) -> Self {
        Self {
            sockfd,
            events,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    /// Deserializes a socket poll request.
    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    /// Returns the networkd socket descriptor.
    pub fn sockfd(&self) -> i32 {
        self.sockfd
    }

    /// Returns the requested events.
    pub fn events(&self) -> c_short {
        self.events
    }

    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    /// Builds a socket poll request message.
    pub fn build(tid: ThreadIdentifier, sockfd: i32, events: c_short) -> Message {
        let request: Self = Self::new(sockfd, events);
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageKind::PollSocketRequest, request.into_bytes());
        Message::new(
            MessageSender::new(ProcessIdentifier::from(i32::from(tid)), tid),
            MessageReceiver::new(crate::NETWORK_DESTINATION, ThreadIdentifier::NONE),
            MessageType::Ikc,
            None,
            message.into_bytes(),
        )
    }
}

/// Response to a non-blocking socket readiness snapshot.
#[derive(Debug)]
#[repr(C, packed)]
pub struct PollSocketResponse {
    revents: c_short,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(PollSocketResponse, SystemCallMessage::PAYLOAD_SIZE);

impl PollSocketResponse {
    const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<c_short>();

    /// Creates a socket poll response.
    pub fn new(revents: c_short) -> Self {
        Self {
            revents,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    /// Deserializes a socket poll response.
    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    /// Returns the reported events.
    pub fn revents(&self) -> c_short {
        self.revents
    }

    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    /// Builds a socket poll response message.
    pub fn build(tid: ThreadIdentifier, revents: c_short) -> Message {
        let response: Self = Self::new(revents);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageKind::PollSocketResponse,
            response.into_bytes(),
        );
        Message::new(
            MessageSender::new(crate::NETWORK_SOURCE, ThreadIdentifier::NONE),
            MessageReceiver::new(ProcessIdentifier::from(i32::from(tid)), tid),
            MessageType::Ikc,
            None,
            message.into_bytes(),
        )
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests request field preservation through serialization.
    #[test]
    fn request_round_trip() {
        let request: PollSocketRequest = PollSocketRequest::new(7, 0x123);
        let decoded: PollSocketRequest = PollSocketRequest::from_bytes(request.into_bytes());
        assert_eq!(decoded.sockfd(), 7);
        assert_eq!(decoded.events(), 0x123);
    }

    /// Tests response field preservation through serialization.
    #[test]
    fn response_round_trip() {
        let response: PollSocketResponse = PollSocketResponse::new(0x45);
        let decoded: PollSocketResponse = PollSocketResponse::from_bytes(response.into_bytes());
        assert_eq!(decoded.revents(), 0x45);
    }
}
