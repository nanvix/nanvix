// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl::message::FileAdvisoryInformationRequest,
    ffi::c_int,
    safe::RawFileDescriptor,
    sys::types::off_t,
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::{
        Error,
        ErrorCode,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Provides advice about the use of a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `offset`: Offset in bytes.
/// - `len`: Length in bytes.
/// - `advice`: Advice to provide.
///
/// # Returns
///
/// Upon success, `posix_fadvise()` empty. Otherwise, it returns an error.
///
pub fn posix_fadvise(
    fd: RawFileDescriptor,
    offset: off_t,
    len: off_t,
    advice: c_int,
) -> Result<(), Error> {
    ::nvx::error!(
        "posix_fadvise(): fd={:?}, offset={:?}, len={:?}, advice={:?}",
        fd,
        offset,
        len,
        advice
    );

    let pid: ProcessIdentifier = ::nvx::pm::getpid()?;

    // Build request and send it.
    let request: Message = FileAdvisoryInformationRequest::build(pid, fd, offset, len, advice);
    ::nvx::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::nvx::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::nvx::error!(
            "posix_fadvise(): failed (fd={:?}, offset={:?}, len={:?}, advice={:?}, status={:?})",
            fd,
            offset,
            len,
            advice,
            { response.status }
        );

        // System call failed, parse error code and return it.
        match ErrorCode::try_from(response.status) {
            // Error code was successfully parsed.
            Ok(error_code) => Err(Error::new(error_code, "posix_fadvise() failed")),
            // Error code was not successfully parsed.
            Err(error) => {
                ::nvx::error!("posix_fadvise(): invalid error code (error={:?})", error);
                Err(Error::new(ErrorCode::TryAgain, "posix_fadvise(): failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::FileAdvisoryInformationResponse => Ok(()),
            header => {
                // Response was not successfully parsed.
                ::nvx::error!(
                    "posix_fadvise(): unexpected message header (fd={:?}, offset={:?}, len={:?}, \
                     advice={:?}, header={:?})",
                    fd,
                    offset,
                    len,
                    advice,
                    header
                );
                Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header"))
            },
        }
    }
}
