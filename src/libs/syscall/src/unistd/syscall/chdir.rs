// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};
#[cfg(not(feature = "standalone"))]
use {
    crate::{
        message::MessagePartitioner,
        unistd::message::ChangeDirectoryRequest,
        LinuxDaemonMessage,
        LinuxDaemonMessageHeader,
    },
    ::alloc::vec::Vec,
    ::sys::{
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
/// Changes the current working directory.
///
/// # Parameters
///
/// - `path`: Pathname of the new working directory.
///
/// # Returns
///
/// Upon successful completion, the `chdir()` system call returns empty. Otherwise, it returns an
/// error.
///
pub fn chdir(path: &str) -> Result<(), Error> {
    ::syslog::trace!("chdir(): path={:?}", path);

    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        ::nvx::vfs::fd::vfs_chdir(path).map_err(|e| {
            let code: ErrorCode = e.into();
            ::syslog::error!("chdir(): VFS chdir failed (path={path:?}, error={e})");
            Error::new(code, "vfs chdir failed")
        })
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    chdir_linuxd(path)
}

/// Forwards a `chdir` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn chdir_linuxd(path: &str) -> Result<(), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    // Build request and send it.
    let request: ChangeDirectoryRequest = ChangeDirectoryRequest::new(path)?;
    let requests: Vec<Message> = request.into_parts(tid)?;
    for request in &requests {
        ::sys::kcall::ipc::send(request)?;
    }

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::error!("chdir(): failed (path={:?}, error_code={:?})", path, { response.status });
        // System call failed, parse error code and return.
        match ErrorCode::try_from(response.status) {
            // Succeeded to parse error code.
            Ok(error_code) => {
                // Return error.
                Err(Error::new(error_code, "chdir() failed"))
            },
            // Failed to parse error code, return generic error.
            Err(error) => {
                ::syslog::error!(
                    "chdir(): failed to parse error code (path={:?}, error={:?})",
                    path,
                    error
                );
                Err(Error::new(ErrorCode::TryAgain, "chdir(): failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        match message.header {
            LinuxDaemonMessageHeader::ChangeDirectoryResponse => Ok(()),
            header => {
                let reason: &str = "unexpected message header";
                ::syslog::error!("chdir(): {:?} (path={:?}, header={:?})", reason, path, header);
                Err(Error::new(ErrorCode::InvalidMessage, reason))
            },
        }
    }
}
