// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::sys_select::{
    fd_set,
    timeval,
    FD_SETSIZE,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Performs synchronous I/O multiplexing.
///
/// # Parameters
///
/// - `nfds`: Highest-numbered file descriptor plus one.
/// - `readfds`: Set of file descriptors to be checked for readability.
/// - `writefds`: Set of file descriptors to be checked for writability.
/// - `errorfds`: Set of file descriptors to be checked for errors.
///
/// # Return Value
///
/// On success, this function returns the number of file descriptors contained in the
/// three returned descriptor sets that are ready for I/O. On failure, an error code is
/// returned instead.
///
pub fn select(
    nfds: usize,
    readfds: Option<&mut fd_set>,
    writefds: Option<&mut fd_set>,
    errorfds: Option<&mut fd_set>,
    timeout: &Option<timeval>,
) -> Result<usize, Error> {
    ::syslog::trace!(
        "select(): nfds={:?}, readfds={:?}, writefds={:?}, errorfds={:?}, timeout={:?}",
        nfds,
        readfds,
        writefds,
        errorfds,
        timeout
    );

    if nfds > FD_SETSIZE {
        return Err(Error::new(
            ErrorCode::InvalidArgument,
            "number of file descriptors exceeds maximum supported",
        ));
    }

    let _ = (readfds, writefds, errorfds, timeout);
    Err(Error::new(ErrorCode::OperationNotSupported, "select not available"))
}
