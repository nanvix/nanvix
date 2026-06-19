// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;

//==================================================================================================
// External Declarations
//==================================================================================================

extern "C" {
    fn _exit(status: c_int) -> !;
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Terminates the calling process. Registered `atexit` handlers are called in reverse order
/// before the process exits.
///
/// POSIX also requires `exit()` to flush all open streams with unwritten buffered data and close
/// their file descriptors before terminating. That flush is not performed here: `libc_stdio` is
/// currently unbuffered (so there is no buffered state to lose), and `libc_stdlib` is linked by
/// guests that do not link `libc_stdio`, so referencing `fflush` directly would break their link.
///
/// # Parameters
///
/// - `status`: Exit status code.
///
/// # Safety
///
/// This function is unsafe because it terminates the process and calls registered handlers.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/exit.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn exit(status: c_int) -> ! {
    crate::atexit::call_atexit_handlers();
    // TODO (#2631): flush and close open stdio streams once `libc_stdio` supports buffering,
    // without creating a hard `libc_stdlib -> libc_stdio` link dependency for non-stdio guests.
    _exit(status)
}

///
/// # Description
///
/// Terminates the calling process without calling `atexit()` handlers or flushing streams.
///
/// # Parameters
///
/// - `status`: Exit status code.
///
/// # Safety
///
/// This function is unsafe because it terminates the process.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/_Exit.html
///
#[allow(non_snake_case)]
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn _Exit(status: c_int) -> ! {
    _exit(status)
}
