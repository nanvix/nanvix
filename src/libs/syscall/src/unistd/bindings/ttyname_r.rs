// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::isatty::isatty;
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::{
        c_char,
        c_int,
    },
    sys_types::c_size_t,
};
use ::syslog::trace_syscall;

//==================================================================================================
// Constants
//==================================================================================================

/// Terminal pathname reported by [`ttyname_r`], including its NUL terminator.
const TTY_NAME: &[u8] = b"/dev/tty\0";

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Stores the pathname of the terminal associated with file descriptor `fd` into the buffer pointed
/// to by `buf`. Nanvix exposes a single console device, so when `fd` refers to a terminal the
/// canonical name `"/dev/tty"` is returned.
///
/// # Parameters
///
/// - `fd`: File descriptor expected to refer to a terminal.
/// - `buf`: Destination buffer that receives the NUL-terminated terminal pathname.
/// - `buflen`: Size of `buf`, in bytes.
///
/// # Returns
///
/// Upon successful completion, `ttyname_r()` returns `0`. If `fd` does not refer to a terminal,
/// `ENOTTY` is returned. If `buflen` is too small to hold the pathname and its terminator, `ERANGE`
/// is returned. The error number is returned directly rather than through `errno`.
///
/// # Safety
///
/// The caller must ensure that `buf` points to a writable region of at least `buflen` bytes.
///
#[allow(clippy::missing_safety_doc)]
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ttyname_r(fd: c_int, buf: *mut c_char, buflen: c_size_t) -> c_int {
    if buf.is_null() {
        return ErrorCode::InvalidArgument.get();
    }

    // Only terminals have a terminal name.
    // SAFETY: `isatty()` performs a kernel query and does not dereference `fd`.
    if unsafe { isatty(fd) } != 1 {
        return ErrorCode::NotTerminal.get();
    }

    let buflen: usize = buflen as usize;
    if buflen < TTY_NAME.len() {
        return ErrorCode::ValueOutOfRange.get();
    }

    // SAFETY: `buf` is non-null and `buflen >= TTY_NAME.len()`, so the copy stays within bounds.
    // The source and destination do not overlap.
    unsafe {
        ::core::ptr::copy_nonoverlapping(TTY_NAME.as_ptr(), buf.cast::<u8>(), TTY_NAME.len());
    }

    0
}
