// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl::message::{
        OpenAtRequest,
        OpenAtResponse,
    },
    message::MessagePartitioner,
    SystemCallMessage,
    SystemCallMessageHeader,
};
use ::alloc::vec::Vec;
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
    sys_types::mode_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn openat(dirfd: i32, pathname: &str, flags: c_int, mode: mode_t) -> Result<c_int, Error> {
    ::syslog::trace!(
        "openat(): dirfd={dirfd:?}, pathname={pathname:?}, flags={flags:?}, mode={mode:?}"
    );

    let pathname: alloc::borrow::Cow<'_, str> = crate::path::expand_path(pathname);
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Build request and send it.
    let request: OpenAtRequest = OpenAtRequest::new(dirfd, &pathname, flags, mode)?;
    let requests: Vec<Message> =
        request.into_parts(tid, crate::VFS_DESTINATION, crate::VFS_MESSAGE_TYPE)?;
    for request in &requests {
        ::sys::kcall::ipc::__kcall_send(request)?;
    }

    // Receive response.
    let response: Message = ::sys::kcall::ipc::__kcall_recv_response()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        match ErrorCode::try_from(response.status) {
            // Succeeded to parse error code.
            Ok(error_code) => {
                ::syslog::warn!(
                    "openat(): failed (dirfd={:?}, pathname={:?}, flags={:?}, mode={:?}, \
                     error={:?})",
                    dirfd,
                    pathname,
                    flags,
                    mode,
                    error_code
                );
                // Return error.
                Err(Error::new(error_code, "openat() failed"))
            },
            // Failed to parse error code, return generic error.
            Err(error) => {
                ::syslog::warn!(
                    "openat(): failed to parse error code (dirfd={:?}, pathname={:?}, flags={:?}, \
                     mode={:?}, error={:?})",
                    dirfd,
                    pathname,
                    flags,
                    mode,
                    error
                );
                Err(Error::new(ErrorCode::TryAgain, "openat(): failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        match SystemCallMessage::try_from_bytes(response.payload) {
            // Response was successfully parsed.
            Ok(message) => match message.header {
                SystemCallMessageHeader::OpenAtResponse => {
                    // Parse response.
                    let response: OpenAtResponse = OpenAtResponse::from_bytes(message.payload);
                    let fd: c_int = response.ret;

                    // Seed the resolution cache with the descriptor vfsd just handed back, stamped
                    // with the table generation vfsd reported, so later descriptor syscalls resolve
                    // it from the cache instead of re-deriving it from the number.
                    if fd >= 0 {
                        // `OpenAtResponse` is `#[repr(C, packed)]`, so `epoch` is not guaranteed to
                        // be aligned. Read it through a raw pointer to avoid forming an unaligned
                        // reference, which is undefined behavior on targets that fault on misaligned
                        // loads.
                        let epoch: u64 =
                            unsafe { ::core::ptr::addr_of!(response.epoch).read_unaligned() };
                        crate::fdtable::record(fd, crate::fdtable::Route::Vfs, fd, epoch);
                    }

                    // Return file descriptor.
                    Ok(fd)
                },
                // Response was not successfully parsed.
                _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
            },
            // Response was not successfully parsed.
            _ => Err(Error::new(ErrorCode::InvalidMessage, "invalid message")),
        }
    }
}
