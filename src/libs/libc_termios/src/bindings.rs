// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::ffi::{
    c_int,
    c_void,
};
use ::syscall::errno::__errno_location;
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Retrieves the parameters associated with the terminal referred to by `fd` and stores them in
/// the structure pointed to by `termios_p`.
///
/// # Parameters
///
/// - `fd`: File descriptor referring to a terminal device.
/// - `termios_p`: Pointer to a buffer where the terminal attributes are stored on success.
///
/// # Returns
///
/// On success, returns `0`. On failure, returns `-1` and sets `errno` to indicate the error.
///
/// # Notes
///
/// In standalone mode this is `ioctl(fd, TCGETS, termios_p)`, backed by the vfsd console terminal.
/// Hosted deployments have no guest terminal device, so the call fails with `ENOSYS`.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers supplied by foreign callers. It is
/// safe to call this function if `termios_p` points to a valid, writable `struct termios`.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn tcgetattr(fd: c_int, termios_p: *mut c_void) -> c_int {
    #[cfg(feature = "standalone")]
    {
        match unsafe { ::syscall::sys::ioctl::ioctl(fd, ::sysapi::sys_ioctl::TCGETS, termios_p) } {
            Ok(_) => 0,
            Err(error) => {
                unsafe {
                    *__errno_location() = error.code.get();
                }
                -1
            },
        }
    }
    #[cfg(not(feature = "standalone"))]
    {
        let _ = (fd, termios_p);
        ::syslog::debug!("tcgetattr(): not implemented");
        unsafe {
            *__errno_location() = ErrorCode::InvalidSysCall.get();
        }
        -1
    }
}

///
/// # Description
///
/// Sets the parameters associated with the terminal referred to by `fd` according to the values
/// in the structure pointed to by `termios_p`.
///
/// # Parameters
///
/// - `fd`: File descriptor referring to a terminal device.
/// - `optional_actions`: How the change is applied (e.g., `TCSANOW`, `TCSADRAIN`, `TCSAFLUSH`).
/// - `termios_p`: Pointer to a buffer containing the desired terminal attributes.
///
/// # Returns
///
/// On success, returns `0`. On failure, returns `-1` and sets `errno` to indicate the error.
///
/// # Notes
///
/// In standalone mode this is `ioctl(fd, TCSETS, termios_p)`, backed by the vfsd console terminal.
/// The console has no output queue to drain or input queue to flush, so `optional_actions` modes
/// (`TCSANOW`/`TCSADRAIN`/`TCSAFLUSH`) are equivalent and the change is applied immediately. Hosted
/// deployments have no guest terminal device, so the call fails with `ENOSYS`.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers supplied by foreign callers. It is
/// safe to call this function if `termios_p` points to a valid, readable `struct termios`.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn tcsetattr(
    fd: c_int,
    optional_actions: c_int,
    termios_p: *const c_void,
) -> c_int {
    #[cfg(feature = "standalone")]
    {
        match optional_actions {
            ::sysapi::termios::TCSANOW
            | ::sysapi::termios::TCSADRAIN
            | ::sysapi::termios::TCSAFLUSH => {},
            _ => {
                unsafe {
                    *__errno_location() = ErrorCode::InvalidArgument.get();
                }
                return -1;
            },
        }
        match unsafe {
            ::syscall::sys::ioctl::ioctl(fd, ::sysapi::sys_ioctl::TCSETS, termios_p as *mut c_void)
        } {
            Ok(_) => 0,
            Err(error) => {
                unsafe {
                    *__errno_location() = error.code.get();
                }
                -1
            },
        }
    }
    #[cfg(not(feature = "standalone"))]
    {
        let _ = (fd, optional_actions, termios_p);
        ::syslog::debug!("tcsetattr(): not implemented");
        unsafe {
            *__errno_location() = ErrorCode::InvalidSysCall.get();
        }
        -1
    }
}
