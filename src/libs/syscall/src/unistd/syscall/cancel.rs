// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use crate::{
    poll::input_message::{
        PipeOpCancelRequest,
        PipeOpCancelResponse,
        PipeOperation,
    },
    SystemCallMessage,
    SystemCallMessageKind,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
        MessageSender,
        RequestToken,
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
    request_id: ::sys::ipc::RequestIdentifier,
) -> Result<Option<u32>, Error> {
    let mut request: Message = PipeOpCancelRequest::build(tid, fd, operation, request_id);
    let token: RequestToken = crate::rpc::send_request(&mut request)?;

    loop {
        let response: Message = match crate::rpc::recv_response_interruptible(&token) {
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
        if response.status != 0 {
            return Err(Error::new(
                ErrorCode::try_from(response.status)?,
                "pipe operation cancellation failed",
            ));
        }
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
        let header: SystemCallMessageKind = message.kind();
        if header != SystemCallMessageKind::PipeOpCancelResponse {
            return Err(Error::new(
                ErrorCode::InvalidMessage,
                "unexpected pipe operation cancellation response",
            ));
        }
        let response: PipeOpCancelResponse = PipeOpCancelResponse::from_bytes(message.payload);
        return if response.cancelled() {
            Ok(Some(response.transferred()))
        } else {
            Ok(None)
        };
    }
}
