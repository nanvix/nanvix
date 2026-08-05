// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::message::{
    self,
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
    WaitCancelResponseMessage,
    WaitResponseMessage,
    WaitTarget,
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

/// Cancels a blocked wait and reports whether cancellation beat completion.
fn cancel_wait(
    caller: ProcessIdentifier,
    request_id: ::sys::ipc::RequestIdentifier,
) -> Result<bool, Error> {
    let mut request: Message = message::wait_cancel_request(caller, request_id)?;
    let token = super::rpc::send_request(&mut request)?;
    let response: Message = super::rpc::recv_response(&token)?;
    if response.message_type != MessageType::Ipc {
        return Err(Error::new(ErrorCode::InvalidMessage, "invalid wait cancellation type"));
    }
    let system: SystemMessage = SystemMessage::from_bytes(response.payload)?;
    if !matches!(system.header, SystemMessageHeader::ProcessManagement) {
        return Err(Error::new(
            ErrorCode::InvalidMessage,
            "invalid wait cancellation system message",
        ));
    }
    let process: ProcessManagementMessage = ProcessManagementMessage::from_bytes(system.payload)?;
    if !matches!(process.header, ProcessManagementMessageHeader::WaitCancelResponse) {
        return Err(Error::new(ErrorCode::InvalidMessage, "unexpected wait cancellation response"));
    }
    Ok(WaitCancelResponseMessage::from_bytes(process.payload).cancelled())
}

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
/// - `target`: Selects which child to wait for. [`WaitTarget::Pid`] waits for that specific child;
///   [`WaitTarget::Any`] waits for any child of the caller.
/// - `options`: Wait options.
///
/// # Returns
///
/// Upon successful completion, the outcome of the wait operation is returned. Upon failure, an
/// error is returned instead.
///
pub fn wait(target: WaitTarget, options: i32) -> Result<WaitOutcome, Error> {
    // Retrieve process identifier of the calling process.
    let caller: ProcessIdentifier = ::sys::kcall::pm::getpid()?;

    // Build wait message and send it.
    let mut message: Message = message::wait_request(caller, target, options)?;
    let token = super::rpc::send_request(&mut message)?;

    // Arbitrate signal interruption against completion without abandoning procd's waiter.
    let message: Message = match super::rpc::recv_response_interruptible(&token) {
        Ok(message) => message,
        Err(error) if error.code == ErrorCode::Interrupted => {
            if cancel_wait(caller, token.identifier())? {
                return Err(error);
            }
            super::rpc::recv_response(&token)?
        },
        Err(error) => return Err(error),
    };

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
                            if child == ProcessIdentifier::from(0) {
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
