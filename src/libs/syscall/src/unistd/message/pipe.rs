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

//==================================================================================================
// PipeRequest
//==================================================================================================

#[repr(C, packed)]
pub struct PipeRequest {
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(PipeRequest, SystemCallMessage::PAYLOAD_SIZE);

impl PipeRequest {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE;

    fn new() -> Self {
        Self {
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
        destination: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: PipeRequest = PipeRequest::new();
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageKind::PipeRequest, message.into_bytes());
        let message: Message = Message::new(
            MessageSender::new(ProcessIdentifier::from(i32::from(tid)), tid),
            MessageReceiver::new(destination, ThreadIdentifier::NONE),
            message_type,
            None,
            message.into_bytes(),
        );

        message
    }
}

//==================================================================================================
// PipeResponse
//==================================================================================================

#[repr(C, packed)]
pub struct PipeResponse {
    pub read_fd: i32,
    pub write_fd: i32,
    /// The `vfsd` table generation at the time these descriptors were allocated.
    ///
    /// libposix stamps both descriptors' resolution-cache entries with this epoch so a later table
    /// mutation can mark them stale.
    pub epoch: u64,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(PipeResponse, SystemCallMessage::PAYLOAD_SIZE);

impl PipeResponse {
    pub const PADDING_SIZE: usize =
        SystemCallMessage::PAYLOAD_SIZE - 2 * mem::size_of::<i32>() - mem::size_of::<u64>();

    fn new(read_fd: i32, write_fd: i32, epoch: u64) -> Self {
        Self {
            read_fd,
            write_fd,
            epoch,
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
        read_fd: i32,
        write_fd: i32,
        epoch: u64,
        source: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: PipeResponse = PipeResponse::new(read_fd, write_fd, epoch);
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageKind::PipeResponse, message.into_bytes());
        let message: Message = Message::new(
            MessageSender::new(source, ThreadIdentifier::NONE),
            MessageReceiver::new(ProcessIdentifier::from(i32::from(tid)), tid),
            message_type,
            None,
            message.into_bytes(),
        );

        message
    }
}
