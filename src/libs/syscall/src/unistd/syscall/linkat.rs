// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::safe::RawFileDescriptor;
use ::sys::error::Error;
use ::sysapi::ffi::c_int;
#[cfg(not(feature = "standalone"))]
use {
    crate::{
        message::MessagePartitioner,
        unistd::message::LinkAtRequest,
        LinuxDaemonMessage,
        LinuxDaemonMessageHeader,
    },
    ::alloc::{
        string::ToString,
        vec::Vec,
    },
    ::sys::{
        error::ErrorCode,
        ipc::Message,
        pm::ThreadIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Creates a new hard link to an existing file relative to a directory file descriptor.
///
/// # Parameters
///
/// - `olddirfd`: Directory file descriptor of the existing file.
/// - `oldpath`: Path to the existing file.
/// - `newdirfd`: Directory file descriptor of the new file.
/// - `newpath`: Path to the new file.
/// - `flags`: Flags to control the behavior of the system call.
///
/// # Returns
///
/// Upon successful completion, `linkat()` returns empty. Otherwise, it returns an error.
///
pub fn linkat(
    olddirfd: RawFileDescriptor,
    oldpath: &str,
    newdirfd: RawFileDescriptor,
    newpath: &str,
    flags: c_int,
) -> Result<(), Error> {
    ::syslog::trace!(
        "linkat(): olddirfd={}, oldpath={}, newdirfd={}, newpath={}, flags={}",
        olddirfd,
        oldpath,
        newdirfd,
        newpath,
        flags
    );

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        ::nvx::vfs::fd::vfs_linkat(olddirfd, oldpath, newdirfd, newpath, flags).map_err(|e| {
            let code: ::sys::error::ErrorCode = e.into();
            ::syslog::error!("linkat(): VFS linkat failed (oldpath={oldpath:?}, error={e})");
            Error::new(code, "vfs linkat failed")
        })
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    linkat_linuxd(olddirfd, oldpath, newdirfd, newpath, flags)
}

/// Forwards a `linkat` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn linkat_linuxd(
    olddirfd: RawFileDescriptor,
    oldpath: &str,
    newdirfd: RawFileDescriptor,
    newpath: &str,
    flags: c_int,
) -> Result<(), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    let request: LinkAtRequest =
        LinkAtRequest::new(olddirfd, oldpath.to_string(), newdirfd, newpath.to_string(), flags)?;

    let requests: Vec<Message> = request.into_parts(tid)?;

    // Send request.
    for request in &requests {
        ::sys::kcall::ipc::send(request)?;
    }

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::error!(
            "linkat(): failed (olddirfd={}, oldpath={}, newdirfd={}, newpath={}, flags={}, \
             error={})",
            olddirfd,
            oldpath,
            newdirfd,
            newpath,
            flags,
            { response.status },
        );
        // System call failed, parse error code and return.
        match ErrorCode::try_from(response.status) {
            // Error code was successfully parsed.
            Ok(error_code) => {
                // Return error.
                Err(Error::new(error_code, "linkat() failed"))
            },
            // Error code was not successfully parsed.
            Err(error) => {
                ::syslog::error!(
                    "linkat(): failed to parse error code (olddirfd={}, oldpath={}, newdirfd={}, \
                     newpath={}, flags={}, error={:?})",
                    olddirfd,
                    oldpath,
                    newdirfd,
                    newpath,
                    flags,
                    error
                );
                Err(Error::new(ErrorCode::TryAgain, "linkat(): failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::LinkAtResponse => Ok(()),
            // Response was not successfully parsed.
            header => {
                let reason: &str = "unexpected message header";
                ::syslog::error!(
                    "linkat(): {:?} (olddirfd={}, oldpath={}, newdirfd={}, newpath={}, flags={}, \
                     header={:?})",
                    reason,
                    olddirfd,
                    oldpath,
                    newdirfd,
                    newpath,
                    flags,
                    header
                );
                Err(Error::new(ErrorCode::InvalidMessage, reason))
            },
        }
    }
}
