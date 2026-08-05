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
        RequestIdentifier,
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

/// VFSD's private pipe-read retry event.
#[derive(Debug)]
#[repr(C, packed)]
pub struct PipeReadRetry {
    pipe_id: u64,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(PipeReadRetry, SystemCallMessage::PAYLOAD_SIZE);

impl PipeReadRetry {
    const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<u64>();

    /// Deserializes a pipe-read retry event.
    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    /// Returns the stable identity of the pipe whose readers should be retried.
    pub fn pipe_id(&self) -> u64 {
        self.pipe_id
    }

    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    /// Builds a retry event addressed to VFSD's process mailbox.
    pub fn build(tid: ThreadIdentifier, pipe_id: u64) -> Message {
        let retry: Self = Self {
            pipe_id,
            _padding: [0; Self::PADDING_SIZE],
        };
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageHeader::PipeReadRetry, retry.into_bytes());
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
    /// Builds a request that cancels one parked console read.
    pub fn build_request(tid: ThreadIdentifier, target: RequestIdentifier) -> Message {
        let mut payload: [u8; SystemCallMessage::PAYLOAD_SIZE] =
            [0u8; SystemCallMessage::PAYLOAD_SIZE];
        payload[..RequestIdentifier::SIZE].copy_from_slice(&target.raw().to_ne_bytes());
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageHeader::ConsoleReadCancelRequest, payload);
        Message::new(
            MessageSender::new(ProcessIdentifier::from(i32::from(tid)), tid),
            MessageReceiver::new(crate::VFS_DESTINATION, ThreadIdentifier::NONE),
            crate::VFS_MESSAGE_TYPE,
            None,
            message.into_bytes(),
        )
    }

    /// Returns the identifier of the parked read targeted by a cancellation request.
    pub fn target(payload: &[u8; SystemCallMessage::PAYLOAD_SIZE]) -> RequestIdentifier {
        RequestIdentifier::from_raw(u32::from_ne_bytes(
            payload[..RequestIdentifier::SIZE]
                .try_into()
                .expect("request identifier slice has a fixed size"),
        ))
    }

    /// Builds an acknowledgement for a console-read cancellation request.
    pub fn build_response(tid: ThreadIdentifier, cancelled: bool) -> Message {
        let mut payload: [u8; SystemCallMessage::PAYLOAD_SIZE] =
            [0u8; SystemCallMessage::PAYLOAD_SIZE];
        payload[0] = u8::from(cancelled);
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageHeader::ConsoleReadCancelResponse, payload);
        Message::new(
            MessageSender::new(ProcessIdentifier::VFSD, ThreadIdentifier::NONE),
            MessageReceiver::new(ProcessIdentifier::from(i32::from(tid)), tid),
            MessageType::Ipc,
            None,
            message.into_bytes(),
        )
    }

    /// Returns whether a parked console read was found and cancelled.
    pub fn cancelled(payload: &[u8; SystemCallMessage::PAYLOAD_SIZE]) -> bool {
        payload[0] != 0
    }
}

/// Kind of pipe operation to cancel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PipeOperation {
    /// A pipe read.
    Read,
    /// A pipe write.
    Write,
}

impl TryFrom<u8> for PipeOperation {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            value if value == Self::Read as u8 => Ok(Self::Read),
            value if value == Self::Write as u8 => Ok(Self::Write),
            _ => Err(()),
        }
    }
}

/// Request to cancel a parked pipe operation.
#[derive(Debug)]
#[repr(C, packed)]
pub struct PipeOpCancelRequest {
    fd: i32,
    operation: u8,
    request_id: u32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(PipeOpCancelRequest, SystemCallMessage::PAYLOAD_SIZE);

impl PipeOpCancelRequest {
    const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE
        - mem::size_of::<i32>()
        - mem::size_of::<u8>()
        - mem::size_of::<u32>();

    /// Creates a pipe-operation cancellation request.
    pub fn new(fd: i32, operation: PipeOperation, request_id: RequestIdentifier) -> Self {
        Self {
            fd,
            operation: operation as u8,
            request_id: request_id.raw(),
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    /// Deserializes a pipe-operation cancellation request.
    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    /// Returns the file descriptor for the cancelled operation.
    pub fn fd(&self) -> i32 {
        self.fd
    }

    /// Returns the kind of operation to cancel, or `None` if the encoded value is invalid.
    pub fn operation(&self) -> Option<PipeOperation> {
        PipeOperation::try_from(self.operation).ok()
    }

    /// Returns the identifier of the parked operation targeted by this request.
    pub fn request_id(&self) -> RequestIdentifier {
        RequestIdentifier::from_raw(self.request_id)
    }

    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    /// Builds a pipe-operation cancellation request.
    pub fn build(
        tid: ThreadIdentifier,
        fd: i32,
        operation: PipeOperation,
        request_id: RequestIdentifier,
    ) -> Message {
        let request: Self = Self::new(fd, operation, request_id);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::PipeOpCancelRequest,
            request.into_bytes(),
        );
        Message::new(
            MessageSender::new(ProcessIdentifier::from(i32::from(tid)), tid),
            MessageReceiver::new(crate::VFS_DESTINATION, ThreadIdentifier::NONE),
            crate::VFS_MESSAGE_TYPE,
            None,
            message.into_bytes(),
        )
    }
}

/// Response to a pipe-operation cancellation request.
#[derive(Debug)]
#[repr(C, packed)]
pub struct PipeOpCancelResponse {
    transferred: u32,
    cancelled: u8,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(PipeOpCancelResponse, SystemCallMessage::PAYLOAD_SIZE);

impl PipeOpCancelResponse {
    const PADDING_SIZE: usize =
        SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<u32>() - mem::size_of::<u8>();

    /// Creates a pipe-operation cancellation response.
    pub fn new(transferred: u32, cancelled: bool) -> Self {
        Self {
            transferred,
            cancelled: u8::from(cancelled),
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    /// Deserializes a pipe-operation cancellation response.
    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    /// Returns the number of bytes transferred before cancellation.
    pub fn transferred(&self) -> u32 {
        self.transferred
    }

    /// Returns whether a parked operation was found and cancelled.
    pub fn cancelled(&self) -> bool {
        self.cancelled != 0
    }

    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    /// Builds a pipe-operation cancellation response.
    pub fn build(tid: ThreadIdentifier, transferred: u32, cancelled: bool) -> Message {
        let response: Self = Self::new(transferred, cancelled);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::PipeOpCancelResponse,
            response.into_bytes(),
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

    /// Tests console-read cancellation outcome preservation through serialization.
    #[test]
    fn console_read_cancel_response_round_trip() {
        for cancelled in [false, true] {
            let response: Message =
                ConsoleReadCancel::build_response(ThreadIdentifier::from(1), cancelled);
            let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)
                .expect("console cancellation response should decode");
            assert_eq!(ConsoleReadCancel::cancelled(&message.payload), cancelled);
        }
    }

    /// Tests pipe cancellation request field preservation through serialization.
    #[test]
    fn pipe_cancel_request_round_trip() {
        let request_id: RequestIdentifier = RequestIdentifier::from_raw(42);
        let request: PipeOpCancelRequest =
            PipeOpCancelRequest::new(7, PipeOperation::Write, request_id);
        let decoded: PipeOpCancelRequest = PipeOpCancelRequest::from_bytes(request.into_bytes());
        assert_eq!(decoded.fd(), 7);
        assert_eq!(decoded.operation(), Some(PipeOperation::Write));
        assert_eq!(decoded.request_id(), request_id);
    }

    /// Tests pipe cancellation response field preservation through serialization.
    #[test]
    fn pipe_cancel_response_round_trip() {
        let response: PipeOpCancelResponse = PipeOpCancelResponse::new(123, true);
        let decoded: PipeOpCancelResponse = PipeOpCancelResponse::from_bytes(response.into_bytes());
        assert_eq!(decoded.transferred(), 123);
        assert!(decoded.cancelled());
    }
}
