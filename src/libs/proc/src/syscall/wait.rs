// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::message::{
    self,
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
    WaitResponseMessage,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
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
/// Outcome of a wait operation.
///
pub enum WaitOutcome {
    /// A child process was reaped.
    Reaped {
        /// Process identifier of the reaped child.
        child: ProcessIdentifier,
        /// Termination status of the reaped child.
        status: i32,
    },
    /// No child process was ready to be reaped. This is only returned when `WNOHANG` is set.
    NoneReady,
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Waits for the termination of a child process (`waitpid()`).
///
/// # Parameters
///
/// - `pid`: Process identifier of the child to wait for (`-1` waits for any child).
/// - `options`: Wait options.
///
/// # Returns
///
/// Upon successful completion, the outcome of the wait operation is returned. Upon failure, an
/// error is returned instead.
///
pub fn wait(pid: ProcessIdentifier, options: i32) -> Result<WaitOutcome, Error> {
    // Retrieve process identifier of the calling process.
    let caller: ProcessIdentifier = ::sys::kcall::pm::__kcall_getpid()?;

    // Build wait message and send it.
    let message: Message = message::wait_request(caller, pid, options)?;
    ::sys::kcall::ipc::__kcall_send(&message)?;

    // Wait response from the process manager daemon.
    let message: Message = ::sys::kcall::ipc::__kcall_recv()?;

    // Parse response.
    match message.message_type {
        MessageType::Ipc => {
            let message: SystemMessage = SystemMessage::from_bytes(message.payload)?;

            // Parse message.
            match message.header {
                // Parse process management message.
                SystemMessageHeader::ProcessManagement => {
                    let message: ProcessManagementMessage =
                        ProcessManagementMessage::from_bytes(message.payload)?;

                    // Parse operation.
                    match message.header {
                        ProcessManagementMessageHeader::WaitResponse => {
                            let message: WaitResponseMessage =
                                WaitResponseMessage::from_bytes(message.payload);

                            // Check for errors.
                            if message.error != 0 {
                                return Err(Error::new(
                                    ErrorCode::try_from(message.error)?,
                                    "failed to wait for child process",
                                ));
                            }

                            // A child process identifier of zero signals that no child was ready.
                            let child: ProcessIdentifier = message.child;
                            if i32::from(child) == 0 {
                                Ok(WaitOutcome::NoneReady)
                            } else {
                                Ok(WaitOutcome::Reaped {
                                    child,
                                    status: message.status,
                                })
                            }
                        },
                        _ => Err(Error::new(
                            ErrorCode::InvalidMessage,
                            "unexpected process management message",
                        )),
                    }
                },
                _ => Err(Error::new(ErrorCode::InvalidMessage, "invalid system message type")),
            }
        },
        _ => Err(Error::new(ErrorCode::InvalidMessage, "invalid message type")),
    }
}
