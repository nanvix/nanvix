// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
    pm::ProcessIdentifier,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A message that drives the `exec` synchronization barrier. It is sent on two hops, distinguished
/// by the kernel-attributed message source:
///
/// - by a freshly `exec`'d process to the process manager daemon, announcing that it has replaced
///   its image and must be held until its inherited descriptor table has had close-on-exec applied;
///   and
/// - by the process manager daemon to the filesystem daemon, asking it to drop the `FD_CLOEXEC`
///   descriptors of `pid` and rebuild its table generation.
///
/// The `pid` it carries names the subject process. On the first hop it is informational (the
/// process manager daemon authoritatively uses the kernel-attributed source instead, so a process
/// can only ever request the barrier for itself); on the second hop it is the subject the
/// filesystem daemon acts on.
///
#[repr(C, packed)]
pub struct ExecMessage {
    /// Process identifier of the subject process (the one that replaced its image).
    pub pid: ProcessIdentifier,
    _padding: [u8; Self::PADDING_SIZE],
}

// NOTE: The size of an exec message must match the size of a process management message payload.
::static_assert::assert_eq_size!(ExecMessage, ProcessManagementMessage::PAYLOAD_SIZE);

///
/// # Description
///
/// A message that acknowledges the `exec` synchronization barrier. It is sent on two hops,
/// distinguished by the kernel-attributed message source:
///
/// - by the filesystem daemon to the process manager daemon, confirming that close-on-exec has been
///   applied to `pid`'s table; and
/// - by the process manager daemon to the subject process, releasing it so that its new image's
///   `crt0` may proceed.
///
/// The `status` field conveys the outcome: `0` when close-on-exec was applied, or a non-zero error
/// code when the barrier could not be completed. A non-zero status lets the held process proceed on
/// a best-effort basis rather than block forever, since its image has already been replaced and the
/// `exec` cannot be undone.
///
#[repr(C, packed)]
pub struct ExecAckMessage {
    /// Process identifier of the subject process the acknowledgement concerns.
    pub pid: ProcessIdentifier,
    /// Outcome of the barrier: `0` on success, or a non-zero error code on failure.
    pub status: i32,
    _padding: [u8; Self::PADDING_SIZE],
}

// NOTE: The size of an exec acknowledgement message must match the size of a process management
// message payload.
::static_assert::assert_eq_size!(ExecAckMessage, ProcessManagementMessage::PAYLOAD_SIZE);

//==================================================================================================
// Implementations
//==================================================================================================

impl ExecMessage {
    /// Size of padding.
    pub const PADDING_SIZE: usize =
        ProcessManagementMessage::PAYLOAD_SIZE - mem::size_of::<ProcessIdentifier>();

    ///
    /// # Description
    ///
    /// Instantiates a new exec message.
    ///
    /// # Parameters
    ///
    /// - `pid`: Process identifier of the subject process.
    ///
    pub fn new(pid: ProcessIdentifier) -> Self {
        Self {
            pid,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    ///
    /// # Description
    ///
    /// Converts a byte array into an exec message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// An exec message.
    ///
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    ///
    /// # Description
    ///
    /// Converts an exec message into a byte array.
    ///
    /// # Returns
    ///
    /// The corresponding byte array.
    ///
    pub fn into_bytes(self) -> [u8; ProcessManagementMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }
}

impl ExecAckMessage {
    /// Status value that marks a successful exec synchronization.
    pub const STATUS_SUCCESS: i32 = 0;

    /// Size of padding.
    pub const PADDING_SIZE: usize = ProcessManagementMessage::PAYLOAD_SIZE
        - mem::size_of::<ProcessIdentifier>()
        - mem::size_of::<i32>();

    ///
    /// # Description
    ///
    /// Instantiates a new exec acknowledgement message.
    ///
    /// # Parameters
    ///
    /// - `pid`: Process identifier of the subject process.
    /// - `status`: Outcome of the barrier (`0` on success, a non-zero error code on failure).
    ///
    pub fn new(pid: ProcessIdentifier, status: i32) -> Self {
        Self {
            pid,
            status,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    ///
    /// # Description
    ///
    /// Converts a byte array into an exec acknowledgement message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// An exec acknowledgement message.
    ///
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    ///
    /// # Description
    ///
    /// Converts an exec acknowledgement message into a byte array.
    ///
    /// # Returns
    ///
    /// The corresponding byte array.
    ///
    pub fn into_bytes(self) -> [u8; ProcessManagementMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Wraps a process management message destined to `destination` into an IPC message sent from
/// `source`.
fn wrap(
    source: ProcessIdentifier,
    destination: ProcessIdentifier,
    pm_message: ProcessManagementMessage,
) -> Message {
    let system_message: SystemMessage =
        SystemMessage::new(SystemMessageHeader::ProcessManagement, pm_message.into_bytes());

    Message::new(
        MessageSender::from(source),
        MessageReceiver::from(destination),
        MessageType::Ipc,
        None,
        system_message.into_bytes(),
    )
}

///
/// # Description
///
/// Builds an exec request message announcing that `pid` has replaced its image.
///
/// # Parameters
///
/// - `source`: Process identifier of the sender (the subject process when announcing to the process
///   manager daemon, or the process manager daemon when notifying the filesystem daemon).
/// - `destination`: Process identifier of the recipient.
/// - `pid`: Process identifier of the subject process.
///
/// # Returns
///
/// Upon successful completion, an exec request message is returned. Otherwise, an error is returned
/// instead.
///
pub fn exec_request(
    source: ProcessIdentifier,
    destination: ProcessIdentifier,
    pid: ProcessIdentifier,
) -> Result<Message, Error> {
    let exec_message: ExecMessage = ExecMessage::new(pid);
    let pm_message: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::Exec,
        exec_message.into_bytes(),
    );

    Ok(wrap(source, destination, pm_message))
}

///
/// # Description
///
/// Builds an exec acknowledgement message for the synchronization barrier of `pid`.
///
/// # Parameters
///
/// - `source`: Process identifier of the sender (the filesystem daemon when confirming to the
///   process manager daemon, or the process manager daemon when releasing the subject process).
/// - `destination`: Process identifier of the recipient.
/// - `pid`: Process identifier of the subject process the acknowledgement concerns.
/// - `status`: Outcome of the barrier (`0` on success, a non-zero error code on failure).
///
/// # Returns
///
/// Upon successful completion, an exec acknowledgement message is returned. Otherwise, an error is
/// returned instead.
///
pub fn exec_ack(
    source: ProcessIdentifier,
    destination: ProcessIdentifier,
    pid: ProcessIdentifier,
    status: i32,
) -> Result<Message, Error> {
    let ack_message: ExecAckMessage = ExecAckMessage::new(pid, status);
    let pm_message: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::ExecAck,
        ack_message.into_bytes(),
    );

    Ok(wrap(source, destination, pm_message))
}
