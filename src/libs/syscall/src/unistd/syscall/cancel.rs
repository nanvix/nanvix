// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use crate::{
    poll::input_message::{
        PipeOpCancelRequest,
        PipeOpCancelResponse,
        PipeOperation,
    },
    SystemCallMessage,
    SystemCallMessageHeader,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
        MessageSender,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

/// Cancels this thread's parked pipe operation and drains the acknowledgement.
pub(super) fn cancel_pipe_operation(
    tid: ThreadIdentifier,
    fd: i32,
    operation: PipeOperation,
) -> Result<u32, Error> {
    let request: Message = PipeOpCancelRequest::build(tid, fd, operation);
    ::sys::kcall::ipc::__kcall_send(&request)?;

    loop {
        let response: Message = match ::sys::kcall::ipc::__kcall_recv() {
            Ok(response) => response,
            Err(error) if error.code == ErrorCode::Interrupted => continue,
            Err(error) => return Err(error),
        };
        let source: MessageSender = response.source;
        if source.pid != ProcessIdentifier::VFSD {
            return Err(Error::new(
                ErrorCode::InvalidMessage,
                "pipe operation cancellation returned an invalid sender",
            ));
        }

        // A response for the interrupted operation may have won the race with the cancellation.
        // Drain it and continue until the cancellation acknowledgement arrives.
        if response.status != 0 {
            continue;
        }
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
        match message.header {
            SystemCallMessageHeader::PipeOpCancelResponse => {
                let response: PipeOpCancelResponse =
                    PipeOpCancelResponse::from_bytes(message.payload);
                return Ok(response.transferred());
            },
            SystemCallMessageHeader::ReadResponse if operation == PipeOperation::Read => continue,
            SystemCallMessageHeader::WriteResponse if operation == PipeOperation::Write => continue,
            _ => {
                return Err(Error::new(
                    ErrorCode::InvalidMessage,
                    "unexpected pipe operation cancellation response",
                ));
            },
        }
    }
}
