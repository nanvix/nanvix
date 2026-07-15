// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Duplicates `oldfd` onto the specific descriptor `newfd`.
///
/// `newfd` is made to alias `oldfd`'s open file description; if `newfd` was already open it is first
/// closed (with the same last-reference accounting as a real `close`). `dup2(fd, fd)` is a no-op
/// that returns `fd`. The operation is performed authoritatively by vfsd's flat slot table, so it
/// works across every backend — including the cross-backend redirections (`dup2(file_fd, 1)`) that
/// the previous split descriptor model could not express.
///
/// Unlike `dup`/`fcntl(F_DUPFD)` — which name no target slot — re-pointing an exact descriptor
/// requires vfsd's authoritative slot table.
pub fn dup2(oldfd: c_int, newfd: c_int) -> Result<c_int, Error> {
    ::syslog::trace!("dup2(): oldfd={:?}, newfd={:?}", oldfd, newfd);
    dup2_via_vfsd(oldfd, newfd)
}

/// Re-points `newfd` at `oldfd` by asking vfsd to perform the slot-table mutation on its
/// authoritative table.
fn dup2_via_vfsd(oldfd: c_int, newfd: c_int) -> Result<c_int, Error> {
    use crate::{
        unistd::message::{
            Dup2Request,
            Dup2Response,
        },
        SystemCallMessage,
        SystemCallMessageHeader,
    };
    use ::sys::{
        ipc::Message,
        pm::ThreadIdentifier,
    };

    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Build request and send it. The descriptors are the caller-facing flat numbers vfsd owns.
    let request: Message =
        Dup2Request::build(tid, oldfd, newfd, crate::VFS_DESTINATION, crate::VFS_MESSAGE_TYPE);
    ::sys::kcall::ipc::__kcall_send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::__kcall_recv()?;

    // Check whether the system call succeeded or not.
    if response.status != 0 {
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        ::syslog::warn!("dup2(): failed (oldfd={oldfd}, newfd={newfd}, error={error_code})");
        return Err(Error::new(error_code, "dup2() failed"));
    }

    // Parse response.
    let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
    match message.header {
        SystemCallMessageHeader::Dup2Response => {
            let response: Dup2Response = Dup2Response::from_bytes(message.payload);
            let ret: c_int = response.ret;
            // `newfd` now aliases `oldfd`'s description, so its routing changed. Drop any cached
            // resolution for it; the next descriptor syscall re-resolves it against vfsd's table.
            crate::fdtable::invalidate(newfd);
            Ok(ret)
        },
        _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
    }
}
