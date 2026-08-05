// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # Job-Control System Calls
//!
//! Client-side helpers that route the POSIX job-control calls — `setsid`, `setpgid`, `getpgid`,
//! `getsid`, `getpgrp`, `tcsetpgrp`, and `tcgetpgrp` — to the process manager daemon, which owns the
//! authoritative session, process-group, and foreground-group state. Each helper sends a single
//! [`crate::message::JobControlRequest`] and decodes the [`crate::message::JobControlResponse`].

//==================================================================================================
// Imports
//==================================================================================================

use crate::message::{
    self,
    JobControlOp,
    JobControlResponse,
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
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
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Routes a job-control operation through the process manager daemon and returns the resulting
/// identifier.
///
/// # Parameters
///
/// - `op`: Operation to perform.
/// - `pid`: Target process identifier (`0` selects the caller).
/// - `pgid`: Process-group argument.
///
/// # Returns
///
/// Upon successful completion, the resulting identifier is returned. Upon failure, an error is
/// returned instead.
///
fn job_control(
    op: JobControlOp,
    pid: ProcessIdentifier,
    pgid: ProcessIdentifier,
) -> Result<ProcessIdentifier, Error> {
    // Retrieve process identifier of the calling process.
    let caller: ProcessIdentifier = ::sys::kcall::pm::getpid()?;

    // Build the job-control request and send it.
    let mut message: Message = message::job_control_request(caller, op, pid, pgid)?;
    let token = super::rpc::send_request(&mut message)?;

    // Wait for the response from the process manager daemon.
    let message: Message = super::rpc::recv_response(&token)?;

    // Parse response.
    match message.message_type {
        MessageType::Ipc => {
            let message: SystemMessage = SystemMessage::from_bytes(message.payload)?;

            match message.header {
                SystemMessageHeader::ProcessManagement => {
                    let message: ProcessManagementMessage =
                        ProcessManagementMessage::from_bytes(message.payload)?;

                    match message.header {
                        ProcessManagementMessageHeader::JobControlResponse => {
                            let response: JobControlResponse =
                                JobControlResponse::from_bytes(message.payload);

                            // Copy fields out of the packed response before use.
                            let error: i32 = response.error;
                            let result: ProcessIdentifier = response.result;

                            if error != 0 {
                                return Err(Error::new(
                                    ErrorCode::try_from(error)?,
                                    "job-control operation failed",
                                ));
                            }

                            Ok(result)
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

///
/// # Description
///
/// Creates a new session and sets the process group of the calling process (`setsid`).
///
/// # Returns
///
/// Upon successful completion, the session identifier of the calling process is returned. Upon
/// failure, an error is returned instead.
///
pub fn setsid() -> Result<ProcessIdentifier, Error> {
    job_control(JobControlOp::SetSid, ProcessIdentifier::from(0), ProcessIdentifier::from(0))
}

///
/// # Description
///
/// Sets the process-group of a process (`setpgid`).
///
/// # Parameters
///
/// - `pid`: Process whose group is set (`0` selects the caller).
/// - `pgid`: New process-group (`0` selects `pid`).
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
pub fn setpgid(pid: ProcessIdentifier, pgid: ProcessIdentifier) -> Result<(), Error> {
    job_control(JobControlOp::SetPgid, pid, pgid).map(|_| ())
}

///
/// # Description
///
/// Returns the process-group of a process (`getpgid`).
///
/// # Parameters
///
/// - `pid`: Process whose group is queried (`0` selects the caller).
///
/// # Returns
///
/// Upon successful completion, the process-group identifier is returned. Upon failure, an error is
/// returned instead.
///
pub fn getpgid(pid: ProcessIdentifier) -> Result<ProcessIdentifier, Error> {
    job_control(JobControlOp::GetPgid, pid, ProcessIdentifier::from(0))
}

///
/// # Description
///
/// Returns the process-group of the calling process (`getpgrp`).
///
/// # Returns
///
/// Upon successful completion, the process-group identifier of the caller is returned. Upon
/// failure, an error is returned instead.
///
pub fn getpgrp() -> Result<ProcessIdentifier, Error> {
    job_control(JobControlOp::GetPgid, ProcessIdentifier::from(0), ProcessIdentifier::from(0))
}

///
/// # Description
///
/// Returns the session of a process (`getsid`).
///
/// # Parameters
///
/// - `pid`: Process whose session is queried (`0` selects the caller).
///
/// # Returns
///
/// Upon successful completion, the session identifier is returned. Upon failure, an error is
/// returned instead.
///
pub fn getsid(pid: ProcessIdentifier) -> Result<ProcessIdentifier, Error> {
    job_control(JobControlOp::GetSid, pid, ProcessIdentifier::from(0))
}

///
/// # Description
///
/// Sets the foreground process group of the controlling terminal (`tcsetpgrp`).
///
/// # Parameters
///
/// - `pgrp`: Process-group to make the foreground group.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
pub fn tcsetpgrp(pgrp: ProcessIdentifier) -> Result<(), Error> {
    job_control(JobControlOp::TcSetPgrp, ProcessIdentifier::from(0), pgrp).map(|_| ())
}

///
/// # Description
///
/// Returns the foreground process group of the controlling terminal (`tcgetpgrp`).
///
/// # Returns
///
/// Upon successful completion, the foreground process-group identifier is returned. Upon failure,
/// an error is returned instead.
///
pub fn tcgetpgrp() -> Result<ProcessIdentifier, Error> {
    job_control(JobControlOp::TcGetPgrp, ProcessIdentifier::from(0), ProcessIdentifier::from(0))
}
