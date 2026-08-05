// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    safe::RawFileDescriptor,
    sys::stat::message::FileChmodRequest,
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
        RequestToken,
    },
    pm::ThreadIdentifier,
};
use sysapi::sys_types::mode_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Changes the mode of a file.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `mode`: Mode of the file.
///
/// # Returns
///
/// Upon successful completion, `fchmod()` returns empty. Otherwise, it returns an error.
///
pub fn fchmod(fd: RawFileDescriptor, mode: mode_t) -> Result<(), Error> {
    ::syslog::trace!("fchmod(): fd={:?}, mode={:o}", fd, mode);

    // Only VFS-backed descriptors are routable here.
    let backend_fd: RawFileDescriptor = {
        use crate::fdtable::{
            resolve_result,
            Route,
        };
        match resolve_result(fd)? {
            Some(res) if res.route == Route::Vfs => res.backend_fd,
            _ => {
                ::syslog::warn!("fchmod(): bad file descriptor fd={fd}");
                return Err(Error::new(ErrorCode::BadFile, "fchmod: fd is not a VFS fd"));
            },
        }
    };

    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Build request and send it
    let mut request: Message = FileChmodRequest::build(
        tid,
        backend_fd,
        mode,
        crate::VFS_DESTINATION,
        crate::VFS_MESSAGE_TYPE,
    );
    let token: RequestToken = crate::rpc::send_request(&mut request)?;

    // Receive response.
    let response: Message = crate::rpc::recv_response(&token)?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::warn!("fchmod(): syscall failed (fd={:?}, mode={:o}, status={:?})", fd, mode, {
            response.status
        });
        // System call failed, parse error code and return it.
        match ErrorCode::try_from(response.status) {
            Ok(error_code) => {
                ::syslog::warn!(
                    "fchmod(): syscall failed (fd={:?}, mode={:o}, error_code={:?})",
                    fd,
                    mode,
                    error_code
                );
                Err(Error::new(error_code, "system call failed"))
            },
            Err(error) => {
                ::syslog::warn!(
                    "fchmod(): syscall failed (fd={:?}, mode={:o}, error={:?})",
                    fd,
                    mode,
                    error
                );
                Err(Error::new(ErrorCode::InvalidMessage, "system call failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
        match message.header {
            // Response was successfully parsed.
            SystemCallMessageHeader::FileChmodResponse => Ok(()),
            // Invalid response.
            header => {
                ::syslog::warn!(
                    "fchmod(): invalid response (fd={:?}, mode={:o}, header={:?})",
                    fd,
                    mode,
                    header
                );
                Err(Error::new(ErrorCode::InvalidMessage, "invalid response"))
            },
        }
    }
}
