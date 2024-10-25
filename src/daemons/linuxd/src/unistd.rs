// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::linuxd::unistd::message::{
    CloseRequest,
    CloseResponse,
    FileDataSyncRequest,
    FileDataSyncResponse,
};
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::ErrorCode,
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

//==================================================================================================
// do_fdatasync
//==================================================================================================

pub fn do_fdatasync(pid: ProcessIdentifier, request: FileDataSyncRequest) -> Message {
    trace!("fdatasync(): pid={:?}, request={:?}", pid, request);

    let fd: i32 = request.fd;

    debug!("libc::fdatasync(): fd={:?}", fd);
    match unsafe { libc::fdatasync(fd) } {
        ret if ret == 0 => FileDataSyncResponse::build(pid, ret),
        ret => crate::build_error(
            pid,
            ErrorCode::try_from(ret).unwrap_or_else(|_| panic!("invalid error code: {:?}", ret)),
        ),
    }
}
