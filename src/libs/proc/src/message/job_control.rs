// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # Job-Control Messages
//!
//! Encodes the requests and responses exchanged between a process and the process manager daemon to
//! manipulate the job-control state the daemon owns: sessions (`setsid`/`getsid`), process groups
//! (`setpgid`/`getpgid`), and the controlling terminal's foreground process group
//! (`tcsetpgrp`/`tcgetpgrp`).
//!
//! All of these collapse onto a single request/response pair tagged by a [`JobControlOp`] opcode,
//! because every one of them is a small fixed-shape transaction: the request names at most a target
//! process and a process-group argument, and the response carries an error code plus a single
//! resulting identifier. Folding them together keeps the process-management message header set small
//! while preserving a distinct, strongly-typed operation per call.

//==================================================================================================
// Imports
//==================================================================================================

use crate::message::{
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
};
use ::core::mem;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
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
// Job-Control Operation
//==================================================================================================

///
/// # Description
///
/// Identifies which job-control operation a [`JobControlRequest`] carries.
///
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobControlOp {
    /// Create a new session (`setsid`): the caller becomes session and process-group leader.
    SetSid = 1,
    /// Set the process-group of a process (`setpgid`).
    SetPgid = 2,
    /// Query the process-group of a process (`getpgid`).
    GetPgid = 3,
    /// Query the session of a process (`getsid`).
    GetSid = 4,
    /// Set the controlling terminal's foreground process group (`tcsetpgrp`).
    TcSetPgrp = 5,
    /// Query the controlling terminal's foreground process group (`tcgetpgrp`).
    TcGetPgrp = 6,
}

impl TryFrom<u8> for JobControlOp {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(JobControlOp::SetSid),
            2 => Ok(JobControlOp::SetPgid),
            3 => Ok(JobControlOp::GetPgid),
            4 => Ok(JobControlOp::GetSid),
            5 => Ok(JobControlOp::TcSetPgrp),
            6 => Ok(JobControlOp::TcGetPgrp),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid job-control operation")),
        }
    }
}

impl From<JobControlOp> for u8 {
    fn from(value: JobControlOp) -> Self {
        value as u8
    }
}

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A message that encodes a job-control request, sent by a process to the process manager daemon.
///
#[repr(C, packed)]
pub struct JobControlRequest {
    /// Operation to perform (a [`JobControlOp`] encoded as a byte).
    pub op: u8,
    /// Target process identifier (`0` selects the caller). Unused by session-wide operations.
    pub pid: ProcessIdentifier,
    /// Process-group argument: the new group for `setpgid`, or the foreground group for `tcsetpgrp`.
    pub pgid: ProcessIdentifier,
    _padding: [u8; Self::PADDING_SIZE],
}

// NOTE: The size of a job-control request must match the size of a process management message payload.
::static_assert::assert_eq_size!(JobControlRequest, ProcessManagementMessage::PAYLOAD_SIZE);

///
/// # Description
///
/// A message that encodes the response to a job-control request.
///
#[repr(C, packed)]
pub struct JobControlResponse {
    /// Error code of the operation (`0` on success).
    pub error: i32,
    /// Resulting identifier (a session, process-group, or foreground-group identifier). Meaningful
    /// only when `error` is `0` and the operation returns an identifier.
    pub result: ProcessIdentifier,
    _padding: [u8; Self::PADDING_SIZE],
}

// NOTE: The size of a job-control response must match the size of a process management message payload.
::static_assert::assert_eq_size!(JobControlResponse, ProcessManagementMessage::PAYLOAD_SIZE);

//==================================================================================================
// Implementations
//==================================================================================================

impl JobControlRequest {
    /// Size of padding.
    pub const PADDING_SIZE: usize = ProcessManagementMessage::PAYLOAD_SIZE
        - mem::size_of::<u8>()
        - mem::size_of::<ProcessIdentifier>()
        - mem::size_of::<ProcessIdentifier>();

    /// Instantiates a new job-control request.
    pub fn new(op: JobControlOp, pid: ProcessIdentifier, pgid: ProcessIdentifier) -> Self {
        Self {
            op: op.into(),
            pid,
            pgid,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    /// Returns the strongly-typed operation carried by this request.
    pub fn op(&self) -> Result<JobControlOp, Error> {
        JobControlOp::try_from(self.op)
    }

    /// Converts a byte array into a job-control request.
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    /// Converts a job-control request into a byte array.
    pub fn into_bytes(self) -> [u8; ProcessManagementMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }
}

impl JobControlResponse {
    /// Size of padding.
    pub const PADDING_SIZE: usize = ProcessManagementMessage::PAYLOAD_SIZE
        - mem::size_of::<i32>()
        - mem::size_of::<ProcessIdentifier>();

    /// Instantiates a new job-control response.
    pub fn new(error: i32, result: ProcessIdentifier) -> Self {
        Self {
            error,
            result,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    /// Converts a byte array into a job-control response.
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    /// Converts a job-control response into a byte array.
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
/// Builds a job-control request message addressed to the process manager daemon.
///
/// # Parameters
///
/// - `caller`: Process identifier of the calling process (and message sender).
/// - `op`: Operation to perform.
/// - `pid`: Target process identifier (`0` selects the caller).
/// - `pgid`: Process-group argument.
///
/// # Returns
///
/// Upon successful completion, a job-control request message is returned. Otherwise, an error is
/// returned instead.
///
pub fn job_control_request(
    caller: ProcessIdentifier,
    op: JobControlOp,
    pid: ProcessIdentifier,
    pgid: ProcessIdentifier,
) -> Result<Message, Error> {
    let job_control_message: JobControlRequest = JobControlRequest::new(op, pid, pgid);

    let pm_message: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::JobControl,
        job_control_message.into_bytes(),
    );

    let system_message: SystemMessage =
        SystemMessage::new(SystemMessageHeader::ProcessManagement, pm_message.into_bytes());

    let ipc_message: Message = Message::new(
        MessageSender::new(caller, ThreadIdentifier::NONE),
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
/// Builds a job-control response message.
///
/// # Parameters
///
/// - `destination`: Destination process.
/// - `error`: Error code of the operation (`0` on success).
/// - `result`: Resulting identifier.
///
/// # Returns
///
/// Upon successful completion, a job-control response message is returned. Otherwise, an error is
/// returned instead.
///
pub fn job_control_response(
    destination: ProcessIdentifier,
    error: i32,
    result: ProcessIdentifier,
) -> Result<Message, Error> {
    let job_control_response_message: JobControlResponse = JobControlResponse::new(error, result);

    let pm_message: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::JobControlResponse,
        job_control_response_message.into_bytes(),
    );

    let system_message: SystemMessage =
        SystemMessage::new(SystemMessageHeader::ProcessManagement, pm_message.into_bytes());

    let ipc_message: Message = Message::new(
        MessageSender::new(ProcessIdentifier::PROCD, ThreadIdentifier::NONE),
        MessageReceiver::new(destination, ThreadIdentifier::NONE),
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
    fn job_control_op_round_trips_through_byte() {
        for op in [
            JobControlOp::SetSid,
            JobControlOp::SetPgid,
            JobControlOp::GetPgid,
            JobControlOp::GetSid,
            JobControlOp::TcSetPgrp,
            JobControlOp::TcGetPgrp,
        ] {
            let byte: u8 = op.into();
            assert_eq!(JobControlOp::try_from(byte).expect("valid op"), op);
        }
    }

    #[test]
    fn job_control_op_rejects_unknown_byte() {
        assert!(JobControlOp::try_from(0).is_err());
        assert!(JobControlOp::try_from(7).is_err());
    }

    #[test]
    fn job_control_request_preserves_fields() {
        let pid: ProcessIdentifier = ProcessIdentifier::from(42);
        let pgid: ProcessIdentifier = ProcessIdentifier::from(7);
        let request: JobControlRequest = JobControlRequest::new(JobControlOp::SetPgid, pid, pgid);
        let decoded: JobControlRequest = JobControlRequest::from_bytes(request.into_bytes());

        assert_eq!(decoded.op().expect("valid op"), JobControlOp::SetPgid);
        assert_eq!({ decoded.pid }, pid);
        assert_eq!({ decoded.pgid }, pgid);
    }

    #[test]
    fn job_control_response_preserves_fields() {
        let result: ProcessIdentifier = ProcessIdentifier::from(123);
        let response: JobControlResponse = JobControlResponse::new(0, result);
        let decoded: JobControlResponse = JobControlResponse::from_bytes(response.into_bytes());

        assert_eq!({ decoded.error }, 0);
        assert_eq!({ decoded.result }, result);
    }
}
