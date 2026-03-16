// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    unistd,
};
use ::sys::error::ErrorCode;
use ::sysapi::ffi::c_int;
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Determines whether a file descriptor refers to a terminal device. The `isatty()` function
/// tests whether the file descriptor `fd` is associated with a terminal device such as a
/// console, pseudo-terminal, or serial line. This function is commonly used by programs to
/// determine if they are running interactively (connected to a terminal) or non-interactively
/// (redirected to a file or pipe). The distinction is important for applications that want to
/// provide different behavior in interactive versus batch mode, such as enabling colored output,
/// progress indicators, or prompting for user input only when connected to a terminal. The
/// function performs this check by examining the file descriptor's properties and determining
/// if it corresponds to a terminal device in the system.
///
/// # Parameters
///
/// - `fd`: File descriptor to test for terminal association. This must be a valid file descriptor
///   that has been opened or is one of the standard file descriptors (stdin=0, stdout=1, stderr=2).
///   The file descriptor can refer to any type of file system object, but the function will only
///   return true for terminal devices. Common file descriptors to test include standard input,
///   output, and error streams to determine if the program is running in an interactive environment.
///
/// # Returns
///
/// The `isatty()` function returns `1` if the file descriptor refers to a terminal device,
/// indicating that the program is running interactively. It returns `0` if the file descriptor
/// does not refer to a terminal (such as when output is redirected to a file or pipe) or if
/// an error occurs during the check. When returning `0` due to an error condition, the function
/// sets `errno` to indicate the specific error, such as `EBADF` for an invalid file descriptor
/// or `ENOTTY` when the file descriptor is valid but does not refer to a terminal device.
/// The return value can be used directly in conditional statements to branch program behavior
/// based on terminal connectivity.
///
/// # Safety
///
/// This function is unsafe because it accesses global state.
///
/// It is safe to use this function if the following conditions are met:
/// - Access to the global `errno` variable is properly synchronized in multithreaded programs.
///
#[trace_libcall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn isatty(fd: c_int) -> c_int {
    match unistd::isatty(fd) {
        Ok(true) => 1,
        Ok(false) => {
            ::syslog::trace!("isatty(): file descriptor is not a terminal (fd={fd:?})");
            *__errno_location() = ErrorCode::NotTerminal.get();
            0
        },
        Err(error) => {
            ::syslog::trace!("isatty(): {error:?}, (fd={fd:?})");
            *__errno_location() = error.code.get();
            0
        },
    }
}
