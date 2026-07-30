// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    safe::RawFileDescriptor,
    unistd::message::{
        SeekRequest,
        SeekResponse,
    },
    SystemCallMessage,
    SystemCallMessageHeader,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ThreadIdentifier,
};
use ::sysapi::{
    ffi::c_int,
    sys_types::off_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn lseek(fd: RawFileDescriptor, offset: off_t, whence: c_int) -> Result<off_t, Error> {
    ::syslog::trace!("lseek(): fd={:?}, offset={}, whence={}", fd, offset, whence);

    // POSIX requires lseek on a pipe/FIFO/socket/stdio fd to return ESPIPE.
    let backend_fd: RawFileDescriptor = {
        use crate::fdtable::{
            resolve_result,
            Route,
        };
        match resolve_result(fd)? {
            // VFS-backed descriptors fall through to the vfsd seek path below.
            Some(res) if res.route == Route::Vfs => res.backend_fd,
            // Seeking the console (stdin/stdout/stderr) is an illegal seek.
            Some(res) if res.route == Route::Console => {
                ::syslog::warn!(
                    "lseek(): illegal seek on stdio (fd={fd:?}, offset={offset}, whence={whence})",
                );
                return Err(Error::new(ErrorCode::IllegalSeek, "illegal seek on stdio"));
            },
            // Sockets and unroutable descriptors are not seekable here.
            _ => {
                ::syslog::warn!("lseek(): bad file descriptor fd={fd}");
                return Err(Error::new(ErrorCode::BadFile, "lseek: fd is not a VFS fd"));
            },
        }
    };

    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Build request and send it.
    let request: Message = SeekRequest::build(
        tid,
        backend_fd,
        offset,
        whence,
        crate::VFS_DESTINATION,
        crate::VFS_MESSAGE_TYPE,
    );
    ::sys::kcall::ipc::__kcall_send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::__kcall_recv_response()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::warn!(
            "lseek(): failed (fd={}, offset={}, whence={}, error={})",
            fd,
            offset,
            whence,
            { response.status },
        );

        // System call failed, parse error code and return.
        match ErrorCode::try_from(response.status) {
            // Error code was successfully parsed.
            Ok(error_code) => {
                // Return error.
                Err(Error::new(error_code, "lseek() failed"))
            },
            // Error code was not successfully parsed.
            Err(error) => {
                ::syslog::warn!(
                    "lseek(): failed to parse error code (fd={}, offset={}, whence={}, error={:?})",
                    fd,
                    offset,
                    whence,
                    error
                );
                Err(Error::new(ErrorCode::InvalidMessage, "failed to parse error code"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: SystemCallMessage = SystemCallMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            SystemCallMessageHeader::SeekResponse => {
                // Parse response.
                let response: SeekResponse = SeekResponse::from_bytes(message.payload);

                Ok(response.offset)
            },
            // Response was not successfully parsed.
            header => {
                ::syslog::warn!(
                    "lseek(): failed to parse response (fd={}, offset={}, whence={}, header={:?})",
                    fd,
                    offset,
                    whence,
                    header
                );
                Err(Error::new(ErrorCode::InvalidMessage, "failed to parse response"))
            },
        }
    }
}
