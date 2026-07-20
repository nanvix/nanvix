// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::{
        c_int,
        c_uint,
        c_void,
    },
    sys_types::pid_t,
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
/// This is `ioctl(fd, TCGETS, termios_p)`, backed by the vfsd console terminal.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers supplied by foreign callers. It is
/// safe to call this function if `termios_p` points to a valid, writable `struct termios`.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn tcgetattr(fd: c_int, termios_p: *mut c_void) -> c_int {
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
/// This is `ioctl(fd, TCSETS, termios_p)`, backed by the vfsd console terminal.
/// The console has no output queue to drain or input queue to flush, so `optional_actions` modes
/// (`TCSANOW`/`TCSADRAIN`/`TCSAFLUSH`) are equivalent and the change is applied immediately.
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

//==================================================================================================
// Terminal line-control and line-speed helpers
//==================================================================================================
//
// These terminal helpers complement `tcgetattr` / `tcsetattr` above. The `tc*` line-control calls
// validate that `fd` is a terminal (via `isatty`) and then complete as no-ops on the queueless vfsd
// console, while the `cf*` accessors read and write the caller's `struct termios` directly.

/// Returns whether `fd` refers to a terminal.
///
/// On a non-terminal or invalid descriptor this sets `errno` (`ENOTTY` or `EBADF`, respectively) as
/// a side effect of `isatty`, so the terminal-control calls below can simply propagate the failure.
fn fd_is_terminal(fd: c_int) -> bool {
    extern "C" {
        fn isatty(fd: c_int) -> c_int;
    }

    // SAFETY: FFI to the runtime `isatty`, which validates `fd` and sets `errno` on failure.
    unsafe { isatty(fd) == 1 }
}

/// Returns whether `speed` is one of the encoded `Bxxx` baud-rate constants.
///
/// Valid encodings are the contiguous low range `B0..=B38400` and the extended range
/// `B57600..=B4000000`. The bare `CBAUDEX` bit (which falls in the gap between the two ranges) and
/// every other value are not baud rates. The bounds are named against `include/termios.h` so the
/// accepted set stays auditable and does not drift from the header encoding.
fn is_baud_rate(speed: c_uint) -> bool {
    use ::sysapi::termios::{
        B0,
        B38400,
        B4000000,
        B57600,
    };

    matches!(speed, B0..=B38400 | B57600..=B4000000)
}

/// `tcsendbreak` — sends a break on the terminal referred to by `fd`.
///
/// The vfsd console has no serial line to signal, so once `fd` is confirmed to be a terminal the
/// break request completes as a successful no-op regardless of `duration`. If `fd` does not refer to
/// a terminal, returns `-1` with `errno` set (`EBADF` or `ENOTTY`), as reported by `isatty`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn tcsendbreak(fd: c_int, _duration: c_int) -> c_int {
    if !fd_is_terminal(fd) {
        return -1;
    }
    0
}

/// `tcdrain` — waits until all output written to the terminal referred to by `fd` is transmitted.
///
/// The vfsd console keeps no output queue to drain, so once `fd` is confirmed to be a terminal the
/// call completes immediately with success. If `fd` does not refer to a terminal, returns `-1` with
/// `errno` set (`EBADF` or `ENOTTY`), as reported by `isatty`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn tcdrain(fd: c_int) -> c_int {
    if !fd_is_terminal(fd) {
        return -1;
    }
    0
}

/// `tcflush` — discards queued terminal data selected by `queue_selector`.
///
/// The descriptor is validated first: if `fd` does not refer to a terminal, returns `-1` with
/// `errno` set (`EBADF` or `ENOTTY`), as reported by `isatty`. Otherwise `queue_selector` must be
/// `TCIFLUSH`, `TCOFLUSH`, or `TCIOFLUSH`; any other value is rejected with `errno = EINVAL`. The
/// vfsd console keeps no output queue and delivers input eagerly, so a valid flush on a terminal is
/// a successful no-op.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn tcflush(fd: c_int, queue_selector: c_int) -> c_int {
    use ::sysapi::termios::{
        TCIFLUSH,
        TCIOFLUSH,
        TCOFLUSH,
    };

    if !fd_is_terminal(fd) {
        return -1;
    }

    match queue_selector {
        TCIFLUSH | TCOFLUSH | TCIOFLUSH => {},
        _ => {
            // SAFETY: writes to the process-global errno location.
            unsafe {
                *__errno_location() = ::sysapi::errno::EINVAL;
            }
            return -1;
        },
    }

    0
}

/// `tcflow` — suspends or resumes terminal input/output as selected by `action`.
///
/// The descriptor is validated first: if `fd` does not refer to a terminal, returns `-1` with
/// `errno` set (`EBADF` or `ENOTTY`), as reported by `isatty`. Otherwise `action` must be `TCOOFF`,
/// `TCOON`, `TCIOFF`, or `TCION`; any other value is rejected with `errno = EINVAL`. The vfsd
/// console applies no software flow control, so a valid action on a terminal is a successful no-op.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn tcflow(fd: c_int, action: c_int) -> c_int {
    use ::sysapi::termios::{
        TCIOFF,
        TCION,
        TCOOFF,
        TCOON,
    };

    if !fd_is_terminal(fd) {
        return -1;
    }

    match action {
        TCOOFF | TCOON | TCIOFF | TCION => {},
        _ => {
            // SAFETY: writes to the process-global errno location.
            unsafe {
                *__errno_location() = ::sysapi::errno::EINVAL;
            }
            return -1;
        },
    }

    0
}

/// `cfgetispeed` — returns the input baud rate stored in `*termios_p`.
///
/// Pure accessor over the caller's `struct termios`: no descriptor, no I/O, and no `errno` contract.
///
/// # Safety
///
/// `termios_p` must point to a valid, readable `struct termios`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn cfgetispeed(termios_p: *const c_void) -> c_uint {
    // SAFETY: the caller guarantees `termios_p` points to a valid, readable `struct termios`.
    let termios: &::sysapi::termios::Termios =
        unsafe { &*(termios_p as *const ::sysapi::termios::Termios) };
    termios.c_ispeed
}

/// `cfgetospeed` — returns the output baud rate stored in `*termios_p`.
///
/// Pure accessor over the caller's `struct termios`: no descriptor, no I/O, and no `errno` contract.
///
/// # Safety
///
/// `termios_p` must point to a valid, readable `struct termios`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn cfgetospeed(termios_p: *const c_void) -> c_uint {
    // SAFETY: the caller guarantees `termios_p` points to a valid, readable `struct termios`.
    let termios: &::sysapi::termios::Termios =
        unsafe { &*(termios_p as *const ::sysapi::termios::Termios) };
    termios.c_ospeed
}

/// `cfsetispeed` — sets the input baud rate stored in `*termios_p`.
///
/// The change is purely in memory and takes effect only when the struct is later handed to
/// `tcsetattr`. `speed` must be one of the `Bxxx` baud-rate constants (`B0` inclusive); any other
/// value is rejected with `errno = EINVAL` and the struct is left unchanged.
///
/// # Safety
///
/// `termios_p` must point to a valid, writable `struct termios`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn cfsetispeed(termios_p: *mut c_void, speed: c_uint) -> c_int {
    if !is_baud_rate(speed) {
        // SAFETY: writes to the process-global errno location.
        unsafe {
            *__errno_location() = ::sysapi::errno::EINVAL;
        }
        return -1;
    }

    // SAFETY: the caller guarantees `termios_p` points to a valid, writable `struct termios`.
    let termios: &mut ::sysapi::termios::Termios =
        unsafe { &mut *(termios_p as *mut ::sysapi::termios::Termios) };
    termios.c_ispeed = speed;
    0
}

/// `cfsetospeed` — sets the output baud rate stored in `*termios_p`.
///
/// The change is purely in memory and takes effect only when the struct is later handed to
/// `tcsetattr`. `speed` must be one of the `Bxxx` baud-rate constants (`B0` inclusive); any other
/// value is rejected with `errno = EINVAL` and the struct is left unchanged.
///
/// # Safety
///
/// `termios_p` must point to a valid, writable `struct termios`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn cfsetospeed(termios_p: *mut c_void, speed: c_uint) -> c_int {
    if !is_baud_rate(speed) {
        // SAFETY: writes to the process-global errno location.
        unsafe {
            *__errno_location() = ::sysapi::errno::EINVAL;
        }
        return -1;
    }

    // SAFETY: the caller guarantees `termios_p` points to a valid, writable `struct termios`.
    let termios: &mut ::sysapi::termios::Termios =
        unsafe { &mut *(termios_p as *mut ::sysapi::termios::Termios) };
    termios.c_ospeed = speed;
    0
}

/// `cfsetspeed` — sets both the input and output baud rates stored in `*termios_p`.
///
/// The change is purely in memory and takes effect only when the struct is later handed to
/// `tcsetattr`. `speed` must be one of the `Bxxx` baud-rate constants (`B0` inclusive); any other
/// value is rejected with `errno = EINVAL` and the struct is left unchanged.
///
/// # Safety
///
/// `termios_p` must point to a valid, writable `struct termios`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn cfsetspeed(termios_p: *mut c_void, speed: c_uint) -> c_int {
    if !is_baud_rate(speed) {
        // SAFETY: writes to the process-global errno location.
        unsafe {
            *__errno_location() = ::sysapi::errno::EINVAL;
        }
        return -1;
    }

    // SAFETY: the caller guarantees `termios_p` points to a valid, writable `struct termios`.
    let termios: &mut ::sysapi::termios::Termios =
        unsafe { &mut *(termios_p as *mut ::sysapi::termios::Termios) };
    termios.c_ispeed = speed;
    termios.c_ospeed = speed;
    0
}

/// `cfmakeraw` — places the terminal attributes pointed to by `termios_p` in "raw" mode.
///
/// Unlike line-speed configuration, this is a pure in-memory transformation of the caller's
/// `struct termios` and requires no terminal hardware. It clears canonical input (`ICANON`), echo
/// (`ECHO`), signal generation (`ISIG`), extended input processing (`IEXTEN`), CR-to-NL mapping
/// (`ICRNL`), start/stop output control (`IXON`), and output post-processing (`OPOST`), then sets a
/// one-byte, no-timeout non-canonical read (`VMIN = 1`, `VTIME = 0`) — the flags honored by the
/// vfsd console line discipline. A null pointer is ignored.
///
/// # Safety
///
/// `termios_p` must be null or point to a valid, writable `struct termios`; a non-null pointer is
/// dereferenced and overwritten.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn cfmakeraw(termios_p: *mut c_void) {
    use ::sysapi::termios::{
        Termios,
        ECHO,
        ICANON,
        ICRNL,
        IEXTEN,
        ISIG,
        IXON,
        OPOST,
        VMIN,
        VTIME,
    };

    if termios_p.is_null() {
        return;
    }

    // SAFETY: the caller guarantees `termios_p` points to a valid, writable `struct termios`.
    let termios: &mut Termios = unsafe { &mut *(termios_p as *mut Termios) };
    termios.c_iflag &= !(ICRNL | IXON);
    termios.c_oflag &= !OPOST;
    termios.c_lflag &= !(ICANON | ECHO | ISIG | IEXTEN);
    termios.c_cc[VMIN] = 1;
    termios.c_cc[VTIME] = 0;
}

/// `tcgetsid` — returns the session ID of the terminal referred to by `fd`.
///
/// Nanvix has no sessions or process groups, so a process is treated as its own session leader:
/// when `fd` refers to a terminal, the returned session ID is the caller's process ID. If `fd` does
/// not refer to a terminal, returns `-1` with `errno` set, as reported by `isatty`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn tcgetsid(fd: c_int) -> pid_t {
    extern "C" {
        fn isatty(fd: c_int) -> c_int;
        fn getpid() -> pid_t;
    }

    // SAFETY: FFI to the runtime `isatty`, which validates `fd` and sets `errno` on error.
    if unsafe { isatty(fd) } == 0 {
        return -1 as pid_t;
    }

    // SAFETY: FFI to the runtime `getpid`, which has no preconditions.
    unsafe { getpid() }
}
