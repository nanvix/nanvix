// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Controlling-terminal detachment notification.

use crate::message::{
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
};
use ::core::mem;
use ::sys::{
    error::Error,
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
        SystemMessage,
        SystemMessageHeader,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

/// A notification that a process created a new session and lost its controlling terminal.
#[repr(C, packed)]
pub struct TerminalDetachMessage {
    /// Process that lost its controlling terminal.
    pub pid: ProcessIdentifier,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(TerminalDetachMessage, ProcessManagementMessage::PAYLOAD_SIZE);

impl TerminalDetachMessage {
    /// Message padding size.
    pub const PADDING_SIZE: usize =
        ProcessManagementMessage::PAYLOAD_SIZE - mem::size_of::<ProcessIdentifier>();

    /// Creates a terminal-detachment notification.
    pub fn new(pid: ProcessIdentifier) -> Self {
        Self {
            pid,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    /// Deserializes a terminal-detachment notification.
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    /// Serializes a terminal-detachment notification.
    pub fn into_bytes(self) -> [u8; ProcessManagementMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }
}

/// Builds a terminal-detachment notification addressed to vfsd.
pub fn terminal_detach_request(pid: ProcessIdentifier) -> Result<Message, Error> {
    let notification: TerminalDetachMessage = TerminalDetachMessage::new(pid);
    let management: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::TerminalDetach,
        notification.into_bytes(),
    );
    let system: SystemMessage =
        SystemMessage::new(SystemMessageHeader::ProcessManagement, management.into_bytes());
    Ok(Message::new(
        MessageSender::new(ProcessIdentifier::PROCD, ThreadIdentifier::NONE),
        MessageReceiver::new(ProcessIdentifier::VFSD, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        system.into_bytes(),
    ))
}
