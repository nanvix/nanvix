// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    SystemCallMessage,
    SystemCallMessageHeader,
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

//==================================================================================================
// Structures
//==================================================================================================

/// Request for an immediate host-console input snapshot.
#[derive(Debug)]
#[repr(C, packed)]
pub struct PollInputRequest {
    count: u32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(PollInputRequest, SystemCallMessage::PAYLOAD_SIZE);

impl PollInputRequest {
    const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<u32>();

    /// No raw input is currently available.
    pub const STATUS_EMPTY: u8 = 0;
    /// The host input stream has reached end-of-file.
    pub const STATUS_EOF: u8 = 1;
    /// Raw input bytes follow the status byte.
    pub const STATUS_DATA: u8 = 2;

    /// Creates a console input poll request.
    pub fn new(count: u32) -> Self {
        Self {
            count,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    /// Deserializes a console input poll request.
    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    /// Returns the maximum number of raw input bytes requested.
    pub fn count(&self) -> u32 {
        self.count
    }

    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    /// Builds a console input poll request message.
    pub fn build(tid: ThreadIdentifier, count: u32) -> Message {
        let request: Self = Self::new(count);
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageHeader::PollInputRequest, request.into_bytes());
        Message::new(
            MessageSender::new(ProcessIdentifier::from(i32::from(tid)), tid),
            MessageReceiver::new(ProcessIdentifier::KERNEL, ThreadIdentifier::NONE),
            MessageType::Ikc,
            None,
            message.into_bytes(),
        )
    }

    /// Builds a VFSD subscription request for console-input availability notifications.
    pub fn build_subscription(tid: ThreadIdentifier) -> Message {
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::ConsoleInputSubscribe,
            [0u8; SystemCallMessage::PAYLOAD_SIZE],
        );
        Message::new(
            MessageSender::new(ProcessIdentifier::VFSD, tid),
            MessageReceiver::new(ProcessIdentifier::KERNEL, ThreadIdentifier::NONE),
            MessageType::Ikc,
            None,
            message.into_bytes(),
        )
    }

    /// Builds a host-to-VFSD notification that console input or EOF is available.
    pub fn build_available_notification() -> Message {
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::ConsoleInputAvailable,
            [0u8; SystemCallMessage::PAYLOAD_SIZE],
        );
        Message::new(
            MessageSender::KERNEL,
            MessageReceiver::VFSD,
            MessageType::Ikc,
            None,
            message.into_bytes(),
        )
    }
}

/// Response to an immediate host-console readiness snapshot.
#[derive(Debug)]
#[repr(C, packed)]
pub struct PollInputResponse {
    status: u8,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(PollInputResponse, SystemCallMessage::PAYLOAD_SIZE);

impl PollInputResponse {
    const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<u8>();

    /// Creates a console input poll response.
    pub fn new(status: u8) -> Self {
        Self {
            status,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    /// Deserializes a console input poll response.
    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    /// Returns the input status.
    pub fn status(&self) -> u8 {
        self.status
    }

    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    /// Builds a console input poll response message.
    pub fn build(tid: ThreadIdentifier, status: u8) -> Message {
        let response: Self = Self::new(status);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::PollInputResponse,
            response.into_bytes(),
        );
        Message::new(
            MessageSender::KERNEL,
            MessageReceiver::new(ProcessIdentifier::from(i32::from(tid)), tid),
            MessageType::Ikc,
            None,
            message.into_bytes(),
        )
    }
}

/// Builds VFSD's private console-read retry event.
pub struct ConsoleReadRetry;

impl ConsoleReadRetry {
    /// Builds a retry event addressed to VFSD's process mailbox.
    pub fn build(tid: ThreadIdentifier) -> Message {
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::ConsoleReadRetry,
            [0u8; SystemCallMessage::PAYLOAD_SIZE],
        );
        Message::new(
            MessageSender::new(ProcessIdentifier::VFSD, tid),
            MessageReceiver::VFSD,
            MessageType::Ipc,
            None,
            message.into_bytes(),
        )
    }
}

/// Builds console-read cancellation protocol messages.
pub struct ConsoleReadCancel;

impl ConsoleReadCancel {
    /// Builds a request that cancels the caller thread's parked console read.
    pub fn build_request(tid: ThreadIdentifier) -> Message {
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::ConsoleReadCancelRequest,
            [0u8; SystemCallMessage::PAYLOAD_SIZE],
        );
        Message::new(
            MessageSender::new(ProcessIdentifier::from(i32::from(tid)), tid),
            MessageReceiver::new(crate::VFS_DESTINATION, ThreadIdentifier::NONE),
            crate::VFS_MESSAGE_TYPE,
            None,
            message.into_bytes(),
        )
    }

    /// Builds an acknowledgement for a console-read cancellation request.
    pub fn build_response(tid: ThreadIdentifier) -> Message {
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::ConsoleReadCancelResponse,
            [0u8; SystemCallMessage::PAYLOAD_SIZE],
        );
        Message::new(
            MessageSender::new(ProcessIdentifier::VFSD, ThreadIdentifier::NONE),
            MessageReceiver::new(ProcessIdentifier::from(i32::from(tid)), tid),
            MessageType::Ipc,
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
        let request: PollInputRequest = PollInputRequest::new(256);
        let decoded: PollInputRequest = PollInputRequest::from_bytes(request.into_bytes());
        assert_eq!(decoded.count(), 256);
    }

    /// Tests response field preservation through serialization.
    #[test]
    fn response_round_trip() {
        let response: PollInputResponse = PollInputResponse::new(PollInputRequest::STATUS_DATA);
        let decoded: PollInputResponse = PollInputResponse::from_bytes(response.into_bytes());
        assert_eq!(decoded.status(), PollInputRequest::STATUS_DATA);
    }
}
