// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sysapi::{
    errno::ENOSYS,
    ffi::{
        c_char,
        c_int,
    },
    sys_types::mode_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Creates a FIFO special file (named pipe). Nanvix does not provide a FIFO subsystem, so this
/// interface is a stub that always fails. It exists so that portable software referencing
/// `mkfifo()` compiles and links; applets that depend on FIFOs are non-functional and documented
/// as gaps.
///
/// # Parameters
///
/// - `path`: Null-terminated path of the FIFO to create.
/// - `mode`: Permission bits for the new FIFO.
///
/// # Returns
///
/// `-1`, with `errno` set to `ENOSYS`.
///
/// # Safety
///
/// This function is unsafe because it writes the thread-local `errno` location.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/mkfifo.html>
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mkfifo(_path: *const c_char, _mode: mode_t) -> c_int {
    unsafe {
        *__errno_location() = ENOSYS;
    }
    -1
}
