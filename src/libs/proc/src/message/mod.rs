// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod exec;
mod fork_clone;
mod fork_sync;
mod job_control;
mod kill;
mod lookup;
mod process_exit;
mod shutdown;
mod signup;
mod terminal;
mod wait;

//==================================================================================================
// Exports
//==================================================================================================

pub use exec::*;
pub use fork_clone::*;
pub use fork_sync::*;
pub use job_control::*;
pub use kill::*;
pub use lookup::*;
pub use process_exit::*;
pub use shutdown::*;
pub use signup::*;
pub use terminal::*;
pub use wait::*;

//==================================================================================================
// Imports
//==================================================================================================

use ::core::mem;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::SystemMessage,
};

//==================================================================================================
// Process Management Message Header
//==================================================================================================

///
/// # Description
///
/// A type that encodes the header of a process management message.
///
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum ProcessManagementMessageHeader {
    /// Shutdown operation.
    Shutdown = 1,
    /// Signup operation.
    Signup = 2,
    /// Signup response.
    SignupResponse = 3,
    /// Lookup operation.
    Lookup = 4,
    /// Lookup response.
    LookupResponse = 5,
    /// Wait operation (a parent asks the process manager daemon to wait for and reap a child).
    Wait = 6,
    /// Wait response (the process manager daemon reports a reaped child's pid and status).
    WaitResponse = 7,
    /// Kill operation (a process asks the process manager daemon to post a signal to a target,
    /// subject to the daemon's permission policy).
    Kill = 8,
    /// Kill response (the process manager daemon reports the outcome of a kill operation).
    KillResponse = 9,
    /// Fork-clone operation (used to notify other daemons to clone a parent's resources onto a
    /// freshly forked child).
    ForkClone = 10,
    /// Fork-sync request (a freshly forked parent asks the process manager daemon to confirm that
    /// the child's filesystem state has been duplicated before either process proceeds).
    ForkSync = 11,
    /// Fork-sync acknowledgement (the process manager daemon releases a parent and its child once
    /// the filesystem daemon has acknowledged the fork-clone snapshot).
    ForkSyncAck = 12,
    /// Process-exit notification (used to notify other daemons to reclaim a terminated process's
    /// per-process state).
    ProcessExit = 13,
    /// Exec notification (a freshly `exec`'d process asks the process manager daemon to apply
    /// close-on-exec to its inherited descriptor table, and the process manager daemon relays this
    /// to the filesystem daemon).
    Exec = 14,
    /// Exec acknowledgement (the filesystem daemon confirms close-on-exec was applied, and the
    /// process manager daemon releases the held process).
    ExecAck = 15,
    /// Fork-clone acknowledgement (the filesystem daemon confirms it has duplicated the parent's
    /// filesystem state onto the freshly forked child, allowing the process manager daemon to
    /// release the held parent and child only once the snapshot has actually been taken rather than
    /// merely dispatched).
    ForkCloneAck = 16,
    /// Job-control operation (a process asks the process manager daemon to manipulate or query
    /// session, process-group, or foreground-group state: `setsid`/`setpgid`/`getpgid`/`getsid`/
    /// `tcsetpgrp`/`tcgetpgrp`).
    JobControl = 17,
    /// Job-control response (the process manager daemon reports the outcome of a job-control
    /// operation).
    JobControlResponse = 18,
    /// Terminal-signal notification (the filesystem daemon asks the process manager daemon to
    /// deliver a terminal-generated signal to the controlling terminal's foreground process group).
    TerminalSignal = 19,
    /// Terminal-access notification (the filesystem daemon reports that a process accessed the
    /// console, so the process manager daemon can raise `SIGTTIN`/`SIGTTOU` for a background group).
    TerminalAccess = 20,
    /// Cancels one blocked wait request by its request identifier.
    WaitCancel = 21,
    /// Reports whether a blocked wait request was cancelled before completion.
    WaitCancelResponse = 22,
}

impl TryFrom<u8> for ProcessManagementMessageHeader {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(ProcessManagementMessageHeader::Shutdown),
            2 => Ok(ProcessManagementMessageHeader::Signup),
            3 => Ok(ProcessManagementMessageHeader::SignupResponse),
            4 => Ok(ProcessManagementMessageHeader::Lookup),
            5 => Ok(ProcessManagementMessageHeader::LookupResponse),
            6 => Ok(ProcessManagementMessageHeader::Wait),
            7 => Ok(ProcessManagementMessageHeader::WaitResponse),
            8 => Ok(ProcessManagementMessageHeader::Kill),
            9 => Ok(ProcessManagementMessageHeader::KillResponse),
            10 => Ok(ProcessManagementMessageHeader::ForkClone),
            11 => Ok(ProcessManagementMessageHeader::ForkSync),
            12 => Ok(ProcessManagementMessageHeader::ForkSyncAck),
            13 => Ok(ProcessManagementMessageHeader::ProcessExit),
            14 => Ok(ProcessManagementMessageHeader::Exec),
            15 => Ok(ProcessManagementMessageHeader::ExecAck),
            16 => Ok(ProcessManagementMessageHeader::ForkCloneAck),
            17 => Ok(ProcessManagementMessageHeader::JobControl),
            18 => Ok(ProcessManagementMessageHeader::JobControlResponse),
            19 => Ok(ProcessManagementMessageHeader::TerminalSignal),
            20 => Ok(ProcessManagementMessageHeader::TerminalAccess),
            21 => Ok(ProcessManagementMessageHeader::WaitCancel),
            22 => Ok(ProcessManagementMessageHeader::WaitCancelResponse),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid process management message")),
        }
    }
}

impl From<&ProcessManagementMessageHeader> for u8 {
    fn from(value: &ProcessManagementMessageHeader) -> Self {
        match value {
            ProcessManagementMessageHeader::Shutdown => 1,
            ProcessManagementMessageHeader::Signup => 2,
            ProcessManagementMessageHeader::SignupResponse => 3,
            ProcessManagementMessageHeader::Lookup => 4,
            ProcessManagementMessageHeader::LookupResponse => 5,
            ProcessManagementMessageHeader::Wait => 6,
            ProcessManagementMessageHeader::WaitResponse => 7,
            ProcessManagementMessageHeader::Kill => 8,
            ProcessManagementMessageHeader::KillResponse => 9,
            ProcessManagementMessageHeader::ForkClone => 10,
            ProcessManagementMessageHeader::ForkSync => 11,
            ProcessManagementMessageHeader::ForkSyncAck => 12,
            ProcessManagementMessageHeader::ProcessExit => 13,
            ProcessManagementMessageHeader::Exec => 14,
            ProcessManagementMessageHeader::ExecAck => 15,
            ProcessManagementMessageHeader::ForkCloneAck => 16,
            ProcessManagementMessageHeader::JobControl => 17,
            ProcessManagementMessageHeader::JobControlResponse => 18,
            ProcessManagementMessageHeader::TerminalSignal => 19,
            ProcessManagementMessageHeader::TerminalAccess => 20,
            ProcessManagementMessageHeader::WaitCancel => 21,
            ProcessManagementMessageHeader::WaitCancelResponse => 22,
        }
    }
}

//==================================================================================================
// Process Management Message
//==================================================================================================

///
/// # Description
///
/// A type that encodes a process management message.
///
#[repr(C, packed)]
pub struct ProcessManagementMessage {
    /// Message header.
    pub header: ProcessManagementMessageHeader,
    /// Request identifier.
    pub request_id: u32,
    /// Message payload.
    pub payload: [u8; Self::PAYLOAD_SIZE],
}

// NOTE: the size of a process management message must match the size of a system message payload.
::static_assert::assert_eq_size!(ProcessManagementMessage, SystemMessage::PAYLOAD_SIZE);

impl ProcessManagementMessage {
    /// Size of payload.
    pub const PAYLOAD_SIZE: usize = SystemMessage::PAYLOAD_SIZE
        - mem::size_of::<ProcessManagementMessageHeader>()
        - mem::size_of::<u32>();

    ///
    /// # Description
    ///
    /// Instantiates a new process management message.
    ///
    /// # Parameters
    ///
    /// - `header`: Message header.
    /// - `payload`: Message payload.
    ///
    /// # Returns
    ///
    /// A process management message.
    ///
    pub fn new(header: ProcessManagementMessageHeader, payload: [u8; Self::PAYLOAD_SIZE]) -> Self {
        Self {
            header,
            request_id: 0,
            payload,
        }
    }

    ///
    /// # Description
    ///
    /// Converts a byte array into a process management message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// A process management message.
    ///
    pub fn from_bytes(bytes: [u8; SystemMessage::PAYLOAD_SIZE]) -> Result<Self, Error> {
        // Check if message header is valid.
        let _header: ProcessManagementMessageHeader =
            ProcessManagementMessageHeader::try_from(bytes[0])?;

        let message: ProcessManagementMessage = unsafe { mem::transmute(bytes) };

        Ok(message)
    }

    ///
    /// # Description
    ///
    /// Converts a process management message into a byte array.
    ///
    /// # Returns
    ///
    /// The corresponding byte array.
    ///
    pub fn into_bytes(self) -> [u8; SystemMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ::sys::ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
        RequestIdentifier,
        SystemMessageHeader,
    };

    #[test]
    fn process_management_request_id_uses_shared_raw_offset() {
        let request_id: u32 = 0x8765_4321;
        let mut process: ProcessManagementMessage = ProcessManagementMessage::new(
            ProcessManagementMessageHeader::Wait,
            [0; ProcessManagementMessage::PAYLOAD_SIZE],
        );
        process.request_id = request_id;
        let system: SystemMessage =
            SystemMessage::new(SystemMessageHeader::ProcessManagement, process.into_bytes());
        let bytes: [u8; Message::PAYLOAD_SIZE] = system.into_bytes();
        assert_eq!(&bytes[2..6], &request_id.to_ne_bytes());

        let mut outer: Message = Message::new(
            MessageSender::KERNEL,
            MessageReceiver::KERNEL,
            MessageType::Ipc,
            None,
            bytes,
        );
        assert_eq!(RequestIdentifier::read_from(&outer).raw(), request_id);

        let replacement: RequestIdentifier = RequestIdentifier::from_raw(0x1234_5678);
        replacement.write_to(&mut outer);

        let system: SystemMessage =
            SystemMessage::from_bytes(outer.payload).expect("system message should decode");
        let decoded: ProcessManagementMessage =
            ProcessManagementMessage::from_bytes(system.payload)
                .expect("process message should decode");
        let decoded_request_id: u32 = decoded.request_id;
        assert_eq!(decoded_request_id, replacement.raw());
    }
}
