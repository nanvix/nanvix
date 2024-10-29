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
use ::nvx::{
    ipc::{
        Message,
        MessageType,
    },
    pm::ProcessIdentifier,
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
::nvx::sys::static_assert_size!(FileStatRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl FileStatRequest {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    /// Creates a new request message.
    fn new(fd: i32) -> Self {
        Self {
            fd,
            padding: [0; Self::PADDING_SIZE],
        }
    }

    /// Creates a new request message from a byte array.
    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    /// Converts the request message to a byte array.
    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(pid: ProcessIdentifier, fd: i32) -> Message {
        let message: FileStatRequest = FileStatRequest::new(fd);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::FileStatRequest,
            message.into_bytes(),
        );
        Message::new(pid, crate::LINUXD, MessageType::Ikc, None, message.into_bytes())
    }
}
