// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::ErrorCode,
};
use linuxd::unistd::message::{
    CloseRequest,
    CloseResponse,
};

//==================================================================================================
// do_close
//==================================================================================================

pub fn do_close(pid: ProcessIdentifier, request: CloseRequest) -> Message {
    trace!("close(): pid={:?}, request={:?}", pid, request);

    let fd: i32 = request.fd;

    debug!("libc::close(): fd={:?}", fd);
    match unsafe { libc::close(fd) } {
        ret if ret == 0 => CloseResponse::build(pid, ret),
        _ => crate::build_error(pid, ErrorCode::InvalidArgument),
    }
}
