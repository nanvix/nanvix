// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::NetError,
    types::LibcMessageFlags,
    NetBackend,
};
use ::log::{
    debug,
    error,
    warn,
};
use ::sys::error::ErrorCode;

//==================================================================================================
// I/O Operations
//==================================================================================================

impl NetBackend {
    /// Sends data on a connected socket.
    ///
    /// Returns the number of bytes sent.
    pub fn send(
        &self,
        sockfd: i32,
        buf: &[u8],
        count: usize,
        flags: i32,
    ) -> Result<isize, NetError> {
        // Validate that count does not exceed buffer length to prevent out-of-bounds reads.
        if count > buf.len() {
            error!("send(): count ({count}) exceeds buffer length ({})", buf.len());
            return Err(NetError::Errno(ErrorCode::InvalidArgument));
        }

        let flags: LibcMessageFlags =
            LibcMessageFlags::try_from_nanvix(flags).map_err(|e| NetError::Errno(e.code))?;

        debug!("libc::send(): sockfd={sockfd:?}, count={count:?}, flags={:?}", flags.inner());

        match unsafe {
            libc::send(sockfd, buf.as_ptr() as *const libc::c_void, count, flags.inner())
        } {
            count if count >= 0 => {
                debug!("libc::send(): count={count:?}");
                Ok(count)
            },
            -1 => {
                let errno: libc::c_int = unsafe { *libc::__errno_location() };
                if errno == libc::EINTR {
                    return Err(NetError::Interrupted);
                }
                error!("libc::send(): failed with errno={errno:?}");
                let error: ErrorCode = ErrorCode::try_from(errno).unwrap_or_else(|_| {
                    warn!("unknown errno value {errno:?}, mapping to ValueOutOfRange");
                    ErrorCode::ValueOutOfRange
                });
                Err(NetError::Errno(error))
            },
            _ => unreachable!("libc::send() returned invalid value"),
        }
    }

    /// Receives data from a connected socket.
    ///
    /// Returns the number of bytes received.
    pub fn recv(
        &self,
        sockfd: i32,
        buf: &mut [u8],
        count: usize,
        flags: i32,
    ) -> Result<isize, NetError> {
        // Validate that count does not exceed buffer length to prevent out-of-bounds writes.
        if count > buf.len() {
            error!("recv(): count ({count}) exceeds buffer length ({})", buf.len());
            return Err(NetError::Errno(ErrorCode::InvalidArgument));
        }

        let flags: LibcMessageFlags =
            LibcMessageFlags::try_from_nanvix(flags).map_err(|e| NetError::Errno(e.code))?;

        debug!("libc::recv(): sockfd={sockfd:?}, len={count:?}, flags={:?}", flags.inner());

        match unsafe {
            libc::recv(sockfd, buf.as_mut_ptr() as *mut libc::c_void, count, flags.inner())
        } {
            count if count >= 0 => {
                debug!("libc::recv(): count={count:?}");
                Ok(count)
            },
            -1 => {
                let errno: libc::c_int = unsafe { *libc::__errno_location() };
                if errno == libc::EINTR {
                    return Err(NetError::Interrupted);
                }
                error!("libc::recv(): failed with errno={errno:?}");
                let error: ErrorCode = ErrorCode::try_from(errno).unwrap_or_else(|_| {
                    warn!("unknown errno value {errno:?}, mapping to ValueOutOfRange");
                    ErrorCode::ValueOutOfRange
                });
                Err(NetError::Errno(error))
            },
            _ => unreachable!("libc::recv() returned invalid value"),
        }
    }
}
