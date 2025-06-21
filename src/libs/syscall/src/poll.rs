// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Structures
//==================================================================================================

#[cfg(feature = "syscall")]
pub mod bindings {

    use sys::error::ErrorCode;

    use ::sysapi::{
        ffi::c_int,
        poll::{
            nfds_t,
            pollfd,
        },
    };

    ///
    /// # Description
    ///
    /// Waits for one of a set of file descriptors to become ready to perform I/O.
    ///
    /// # Parameters
    ///
    /// - `fds`: Pointer to an array of pollfd structures describing the file descriptors to poll.
    /// - `nfds`: Number of file descriptors in the array.
    /// - `timeout`: Timeout in milliseconds. A negative value means infinite timeout.
    ///
    /// # Returns
    ///
    /// Returns the number of file descriptors with events, `0` if timed out, or `-1` on error.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it may dereference raw pointers.
    ///
    /// It is safe to call this function if the following conditions are met:
    /// - `fds` points to a valid array of pollfd structures of length `nfds`.
    ///
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn poll(fds: *mut pollfd, nfds: nfds_t, timeout: c_int) -> c_int {
        let fds = core::slice::from_raw_parts(fds, nfds as usize);

        ::syslog::trace!("poll(): fds={fds:?}, nfds={nfds:?}, timeout={timeout:?}");
        let ret = ErrorCode::Interrupted.get();

        ::syslog::error!("poll(): not implemented, returning {ret:?}");

        ret
    }
}
