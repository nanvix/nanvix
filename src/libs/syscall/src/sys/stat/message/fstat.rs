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
// FileStatRequest
//==================================================================================================

///
/// # Description
///
/// This struct represents the request message of the `fstat()` system call.
///
#[derive(Debug)]
#[repr(C, packed)]
pub struct FileStatRequest {
    /// File descriptor.
    pub fd: i32,
    /// Padding.
    pub padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(FileStatRequest, SystemCallMessage::PAYLOAD_SIZE);

impl FileStatRequest {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    /// Creates a new request message.
    fn new(fd: i32) -> Self {
        Self {
            fd,
            padding: [0; Self::PADDING_SIZE],
        }
    }

    /// Creates a new request message from a byte array.
    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    /// Converts the request message to a byte array.
    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(
        tid: ThreadIdentifier,
        fd: i32,
        destination: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: FileStatRequest = FileStatRequest::new(fd);
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageKind::FileStatRequest, message.into_bytes());
        Message::new(
            MessageSender::new(ProcessIdentifier::from(i32::from(tid)), tid),
            MessageReceiver::new(destination, ThreadIdentifier::NONE),
            message_type,
            None,
            message.into_bytes(),
        )
    }
}
