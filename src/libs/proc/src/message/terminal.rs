// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # Terminal Notifications
//!
//! Encodes the fire-and-forget notifications sent to the process manager daemon (the owner of job
//! control) so that terminal-driven events reach the right process group:
//!
//! - [`TerminalSignalMessage`] asks the daemon to deliver a terminal-generated signal (`SIGINT` from
//!   `^C`, `SIGTSTP` from `^Z`, `SIGQUIT` from `^\`) to the controlling terminal's *foreground*
//!   process group.
//! - [`TerminalAccessMessage`] reports that a process attempted to read from (or write to) the
//!   console, so the daemon can raise `SIGTTIN`/`SIGTTOU` when that process is in a *background*
//!   group. Console reads are reported by the filesystem daemon, which owns the shared input path;
//!   direct console writes may be reported by the writing process for itself.
//!
//! Neither notification carries a reply: senders do not block on the process manager daemon,
//! exactly as the existing fork/exit notifications do not.

//==================================================================================================
// Imports
//==================================================================================================

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

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A notification asking the process manager daemon to deliver a terminal-generated signal to the
/// controlling terminal's foreground process group.
///
#[repr(C, packed)]
pub struct TerminalSignalMessage {
    /// Signal number to deliver to the foreground process group.
    pub signum: i32,
    _padding: [u8; Self::PADDING_SIZE],
}

// NOTE: The size of a terminal-signal message must match the size of a process management message payload.
::static_assert::assert_eq_size!(TerminalSignalMessage, ProcessManagementMessage::PAYLOAD_SIZE);

///
/// # Description
///
/// A notification reporting that a process accessed the console while possibly in a background
/// process group, so the daemon can raise `SIGTTIN` (read) or `SIGTTOU` (write) as required.
///
#[repr(C, packed)]
pub struct TerminalAccessMessage {
    /// Process that accessed the console.
    pub pid: ProcessIdentifier,
    /// Non-zero when the access was a write (`SIGTTOU`); zero for a read (`SIGTTIN`).
    pub write: u8,
    _padding: [u8; Self::PADDING_SIZE],
}

// NOTE: The size of a terminal-access message must match the size of a process management message payload.
::static_assert::assert_eq_size!(TerminalAccessMessage, ProcessManagementMessage::PAYLOAD_SIZE);

//==================================================================================================
// Implementations
//==================================================================================================

impl TerminalSignalMessage {
    /// Size of padding.
    pub const PADDING_SIZE: usize = ProcessManagementMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    /// Instantiates a new terminal-signal message.
    pub fn new(signum: i32) -> Self {
        Self {
            signum,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    /// Converts a byte array into a terminal-signal message.
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    /// Converts a terminal-signal message into a byte array.
    pub fn into_bytes(self) -> [u8; ProcessManagementMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }
}

impl TerminalAccessMessage {
    /// Size of padding.
    pub const PADDING_SIZE: usize = ProcessManagementMessage::PAYLOAD_SIZE
        - mem::size_of::<ProcessIdentifier>()
        - mem::size_of::<u8>();

    /// Instantiates a new terminal-access message.
    pub fn new(pid: ProcessIdentifier, write: bool) -> Self {
        Self {
            pid,
            write: write as u8,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    /// Returns `true` when the reported access was a write.
    pub fn is_write(&self) -> bool {
        self.write != 0
    }

    /// Converts a byte array into a terminal-access message.
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    /// Converts a terminal-access message into a byte array.
    pub fn into_bytes(self) -> [u8; ProcessManagementMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Builds a terminal-signal notification addressed to the process manager daemon.
///
/// # Parameters
///
/// - `sender`: Process identifier of the sender (the filesystem daemon).
/// - `signum`: Signal number to deliver to the foreground process group.
///
/// # Returns
///
/// Upon successful completion, a terminal-signal notification message is returned. Otherwise, an
/// error is returned instead.
///
pub fn terminal_signal_request(sender: ProcessIdentifier, signum: i32) -> Result<Message, Error> {
    let terminal_signal_message: TerminalSignalMessage = TerminalSignalMessage::new(signum);

    let pm_message: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::TerminalSignal,
        terminal_signal_message.into_bytes(),
    );

    let system_message: SystemMessage =
        SystemMessage::new(SystemMessageHeader::ProcessManagement, pm_message.into_bytes());

    let ipc_message: Message = Message::new(
        MessageSender::new(sender, ThreadIdentifier::NONE),
        MessageReceiver::new(ProcessIdentifier::PROCD, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        system_message.into_bytes(),
    );

    Ok(ipc_message)
}

///
/// # Description
///
/// Builds a terminal-access notification addressed to the process manager daemon.
///
/// # Parameters
///
/// - `sender`: Process identifier of the sender (the filesystem daemon for reads, or the writing
///   process for direct console writes).
/// - `pid`: Process that accessed the console.
/// - `write`: `true` for a write (`SIGTTOU`), `false` for a read (`SIGTTIN`).
///
/// # Returns
///
/// Upon successful completion, a terminal-access notification message is returned. Otherwise, an
/// error is returned instead.
///
pub fn terminal_access_request(
    sender: ProcessIdentifier,
    pid: ProcessIdentifier,
    write: bool,
) -> Result<Message, Error> {
    let terminal_access_message: TerminalAccessMessage = TerminalAccessMessage::new(pid, write);

    let pm_message: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::TerminalAccess,
        terminal_access_message.into_bytes(),
    );

    let system_message: SystemMessage =
        SystemMessage::new(SystemMessageHeader::ProcessManagement, pm_message.into_bytes());

    let ipc_message: Message = Message::new(
        MessageSender::new(sender, ThreadIdentifier::NONE),
        MessageReceiver::new(ProcessIdentifier::PROCD, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        system_message.into_bytes(),
    );

    Ok(ipc_message)
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_signal_message_preserves_signum() {
        let message: TerminalSignalMessage = TerminalSignalMessage::new(2);
        let decoded: TerminalSignalMessage =
            TerminalSignalMessage::from_bytes(message.into_bytes());
        assert_eq!({ decoded.signum }, 2);
    }

    #[test]
    fn terminal_access_message_preserves_fields() {
        let pid: ProcessIdentifier = ProcessIdentifier::from(7);

        let read: TerminalAccessMessage = TerminalAccessMessage::new(pid, false);
        let read: TerminalAccessMessage = TerminalAccessMessage::from_bytes(read.into_bytes());
        assert_eq!({ read.pid }, pid);
        assert!(!read.is_write());

        let write: TerminalAccessMessage = TerminalAccessMessage::new(pid, true);
        let write: TerminalAccessMessage = TerminalAccessMessage::from_bytes(write.into_bytes());
        assert_eq!({ write.pid }, pid);
        assert!(write.is_write());
    }
}
