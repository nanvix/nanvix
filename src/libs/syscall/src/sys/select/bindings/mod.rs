// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    sys::select::syscall,
};
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sys_select::{
        fd_set,
        timeval,
        FD_SETSIZE,
    },
};
use ::syslog::trace_syscall;

//==================================================================================================
// Structures
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
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
///
/// It is safe to call this function if and only if the following conditions are met:
/// - If `readfds` is not null, it points to a valid `fd_set` structure.
/// - If `writefds` is not null, it points to a valid `fd_set` structure.
/// - If `errorfds` is not null, it points to a valid `fd_set` structure.
/// - If `timeout` is not null, it points to a valid `timeval` structure
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn select(
    nfds: c_int,
    readfds: *mut fd_set,
    writefds: *mut fd_set,
    errorfds: *mut fd_set,
    timeout: *const timeval,
) -> c_int {
    // Check if `nfds` is not valid.
    if nfds < 0 || nfds as usize > FD_SETSIZE {
        ::syslog::error!(
            "select(): invalid nfds (nfds={nfds:?}, readfds={readfds:?}, writefds={writefds:?}, \
             errorfds={errorfds:?})"
        );
        // SAFETY: `__errno_location()` returns a valid pointer.
        unsafe {
            *__errno_location() = ErrorCode::InvalidArgument.get();
        }
        return -1;
    }

    // Build references to fd_set structures (do not modify input sets here).
    let read_ref: Option<&mut fd_set> = if readfds.is_null() {
        None
    } else {
        // Check if pointer is misaligned.
        if !(readfds as usize).is_multiple_of(core::mem::align_of::<fd_set>()) {
            ::syslog::error!("select(): misaligned readfds pointer (readfds={readfds:?})");
            // SAFETY: `__errno_location()` returns a valid pointer.
            unsafe {
                *__errno_location() = ErrorCode::InvalidArgument.get();
            }
            return -1;
        }

        unsafe { Some(&mut *readfds) }
    };
    let write_ref: Option<&mut fd_set> = if writefds.is_null() {
        None
    } else {
        // Check if pointer is misaligned.
        if !(writefds as usize).is_multiple_of(core::mem::align_of::<fd_set>()) {
            ::syslog::error!("select(): misaligned writefds pointer (writefds={writefds:?})");
            // SAFETY: `__errno_location()` returns a valid pointer.
            unsafe {
                *__errno_location() = ErrorCode::InvalidArgument.get();
            }
            return -1;
        }

        unsafe { Some(&mut *writefds) }
    };
    let error_ref: Option<&mut fd_set> = if errorfds.is_null() {
        None
    } else {
        // Check if pointer is misaligned.
        if !(errorfds as usize).is_multiple_of(core::mem::align_of::<fd_set>()) {
            ::syslog::error!("select(): misaligned errorfds pointer (errorfds={errorfds:?})");
            // SAFETY: `__errno_location()` returns a valid pointer.
            unsafe {
                *__errno_location() = ErrorCode::InvalidArgument.get();
            }
            return -1;
        }

        unsafe { Some(&mut *errorfds) }
    };

    // Convert `timeout`.
    let timeout: Option<timeval> = if timeout.is_null() {
        None
    } else {
        // SAFETY: timeout is not null.
        Some(unsafe { timeout.read_unaligned() })
    };

    match syscall::select(nfds as usize, read_ref, write_ref, error_ref, &timeout) {
        Ok(ready_fds) => ready_fds as c_int,
        Err(err) => {
            ::syslog::error!("select(): syscall failed (nfds={nfds:?}, error={err:?})");
            // SAFETY: `__errno_location()` returns a valid pointer.
            unsafe {
                *__errno_location() = err.code.get();
            }
            -1
        },
    }
}
