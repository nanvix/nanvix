// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Controlling-terminal detachment protocol.

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

/// A request to detach a process from its controlling terminal.
#[repr(C, packed)]
pub struct TerminalDetachMessage {
    /// Process that is creating a new session.
    pub pid: ProcessIdentifier,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(TerminalDetachMessage, ProcessManagementMessage::PAYLOAD_SIZE);

/// An acknowledgement that a controlling-terminal detachment was applied.
#[repr(C, packed)]
pub struct TerminalDetachAckMessage {
    /// Process whose terminal association was updated.
    pub pid: ProcessIdentifier,
    /// Outcome of the detachment: zero on success, or an error code on failure.
    pub status: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(TerminalDetachAckMessage, ProcessManagementMessage::PAYLOAD_SIZE);

impl TerminalDetachMessage {
    /// Message padding size.
    pub const PADDING_SIZE: usize =
        ProcessManagementMessage::PAYLOAD_SIZE - mem::size_of::<ProcessIdentifier>();

    /// Creates a terminal-detachment request.
    pub fn new(pid: ProcessIdentifier) -> Self {
        Self {
            pid,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    /// Deserializes a terminal-detachment request.
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    /// Serializes a terminal-detachment request.
    pub fn into_bytes(self) -> [u8; ProcessManagementMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }
}

impl TerminalDetachAckMessage {
    /// Status value that marks a successful detachment.
    pub const STATUS_SUCCESS: i32 = 0;

    /// Message padding size.
    pub const PADDING_SIZE: usize = ProcessManagementMessage::PAYLOAD_SIZE
        - mem::size_of::<ProcessIdentifier>()
        - mem::size_of::<i32>();

    /// Creates a terminal-detachment acknowledgement.
    pub fn new(pid: ProcessIdentifier, status: i32) -> Self {
        Self {
            pid,
            status,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    /// Deserializes a terminal-detachment acknowledgement.
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    /// Serializes a terminal-detachment acknowledgement.
    pub fn into_bytes(self) -> [u8; ProcessManagementMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }
}

/// Wraps a terminal-detachment message in an IPC message.
fn wrap(
    source: ProcessIdentifier,
    destination: ProcessIdentifier,
    header: ProcessManagementMessageHeader,
    payload: [u8; ProcessManagementMessage::PAYLOAD_SIZE],
) -> Message {
    let management: ProcessManagementMessage = ProcessManagementMessage::new(header, payload);
    let system: SystemMessage =
        SystemMessage::new(SystemMessageHeader::ProcessManagement, management.into_bytes());
    Message::new(
        MessageSender::new(source, ThreadIdentifier::NONE),
        MessageReceiver::new(destination, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        system.into_bytes(),
    )
}

/// Builds a terminal-detachment request addressed to vfsd.
pub fn terminal_detach_request(pid: ProcessIdentifier) -> Result<Message, Error> {
    let request: TerminalDetachMessage = TerminalDetachMessage::new(pid);
    Ok(wrap(
        ProcessIdentifier::PROCD,
        ProcessIdentifier::VFSD,
        ProcessManagementMessageHeader::TerminalDetach,
        request.into_bytes(),
    ))
}

/// Builds a terminal-detachment acknowledgement addressed to procd.
pub fn terminal_detach_ack(pid: ProcessIdentifier, status: i32) -> Result<Message, Error> {
    let acknowledgement: TerminalDetachAckMessage = TerminalDetachAckMessage::new(pid, status);
    Ok(wrap(
        ProcessIdentifier::VFSD,
        ProcessIdentifier::PROCD,
        ProcessManagementMessageHeader::TerminalDetachAck,
        acknowledgement.into_bytes(),
    ))
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_detach_request_round_trip() {
        let pid: ProcessIdentifier = ProcessIdentifier::from(42);
        let request: TerminalDetachMessage = TerminalDetachMessage::new(pid);
        let decoded: TerminalDetachMessage =
            TerminalDetachMessage::from_bytes(request.into_bytes());

        assert_eq!({ decoded.pid }, pid);
    }

    #[test]
    fn terminal_detach_ack_round_trip() {
        let pid: ProcessIdentifier = ProcessIdentifier::from(42);
        let status: i32 = -1;
        let acknowledgement: TerminalDetachAckMessage = TerminalDetachAckMessage::new(pid, status);
        let decoded: TerminalDetachAckMessage =
            TerminalDetachAckMessage::from_bytes(acknowledgement.into_bytes());

        assert_eq!({ decoded.pid }, pid);
        assert_eq!({ decoded.status }, status);
    }
}
