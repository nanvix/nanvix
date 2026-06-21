// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Client-side `ioctl` for terminal-control requests.
//!
//! Terminal probes (`TCGETS`/`TCSETS`/`TIOCGWINSZ`/`TIOCSWINSZ`) are routed to the descriptor's
//! owning backend: when the descriptor resolves to the console backend the request is forwarded to
//! vfsd, which owns the shared terminal state. Every other request keeps the historical permissive
//! behavior of returning success without acting, so non-terminal callers are unaffected.
//!
//! The `termios`/`winsize` payload does not fit in a single IPC message, so it is transferred out of
//! band via the push/pull rendezvous (the same mechanism `read`/`write` use): a *get* pulls the
//! payload from vfsd, a *set* pushes the payload to vfsd.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;
#[cfg(feature = "standalone")]
use ::sys::error::ErrorCode;
use ::sysapi::ffi::{
    c_int,
    c_void,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Performs a control operation on the device referred to by `fd`.
///
/// Terminal-control requests on a console descriptor are routed to vfsd; every other request
/// returns `0` without acting.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `request`: The device-dependent request code.
/// - `arg`: Pointer to the request's argument (for terminal requests, a `termios` or `winsize`).
///
/// # Returns
///
/// Upon successful completion, `0` is returned. Otherwise, an error is returned.
///
/// # Safety
///
/// For terminal-control requests, `arg` must point to a valid object of the type the request
/// expects (`struct termios` for `TCGETS`/`TCSETS`, `struct winsize` for `TIOCGWINSZ`/`TIOCSWINSZ`).
pub unsafe fn ioctl(fd: c_int, request: c_int, arg: *mut c_void) -> Result<c_int, Error> {
    #[cfg(feature = "standalone")]
    {
        ioctl_standalone(fd, request, arg)
    }
    #[cfg(not(feature = "standalone"))]
    {
        // Hosted deployments have no vfsd console backend to answer terminal requests; preserve the
        // historical permissive behavior of ignoring the request.
        let _ = (fd, arg);
        ::syslog::debug!(
            "ioctl(): not implemented in hosted mode, ignoring (request={request:#x})"
        );
        Ok(0)
    }
}

/// Routes a terminal-control request to vfsd, or ignores a non-terminal request.
#[cfg(feature = "standalone")]
fn ioctl_standalone(fd: c_int, request: c_int, arg: *mut c_void) -> Result<c_int, Error> {
    use ::sysapi::{
        sys_ioctl::{
            Winsize,
            TCGETS,
            TCSETS,
            TIOCGWINSZ,
            TIOCSWINSZ,
        },
        termios::Termios,
    };

    // Classify the request by payload length.
    let len: usize = match request {
        TCGETS | TCSETS => Termios::SIZE,
        TIOCGWINSZ | TIOCSWINSZ => Winsize::SIZE,
        // Not a terminal-control request: keep the historical permissive behavior.
        _ => {
            ::syslog::debug!("ioctl(): unsupported request {request:#x}, ignoring (fd={fd})");
            return Ok(0);
        },
    };

    // A terminal-control request requires a valid user buffer to read from or write into.
    if arg.is_null() {
        ::syslog::warn!("ioctl(): null argument pointer (fd={fd}, request={request:#x})");
        return Err(Error::new(ErrorCode::BadAddress, "ioctl: null argument pointer"));
    }

    // Only a console descriptor is a terminal; a non-console fd is ENOTTY, an unknown fd is EBADF.
    let tty_fd: i32 = crate::fdtable::resolve_tty(fd, "ioctl")?;

    match request {
        TCGETS | TIOCGWINSZ => tty_pull(tty_fd, request, arg as *mut u8, len),
        TCSETS => {
            let termios: &Termios = unsafe { &*(arg as *const Termios) };
            tty_push(tty_fd, request, &termios.to_bytes())
        },
        TIOCSWINSZ => {
            let winsize: &Winsize = unsafe { &*(arg as *const Winsize) };
            tty_push(tty_fd, request, winsize.as_bytes())
        },
        _ => unreachable!("terminal-control request was classified above"),
    }
}

/// Fetches a terminal attribute payload from vfsd into `out` (a *get* request).
///
/// Sends the metadata request, pulls the `len`-byte payload that vfsd pushes, then receives the
/// status response — mirroring the `read` bulk-transfer protocol.
#[cfg(feature = "standalone")]
fn tty_pull(fd: i32, request: c_int, out: *mut u8, len: usize) -> Result<c_int, Error> {
    use crate::sys::ioctl::message::TtyControlRequest;
    use ::sys::ipc::Message;

    let tid: ::sys::pm::ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    // Safety: `out` is non-null (checked by the caller) and points to a `len`-byte object the caller
    // owns; vfsd pushes exactly `len` bytes into it.
    let out: &mut [u8] = unsafe { ::core::slice::from_raw_parts_mut(out, len) };

    let req: Message = TtyControlRequest::build(
        tid,
        fd,
        request,
        len as u32,
        crate::VFS_DESTINATION,
        crate::VFS_MESSAGE_TYPE,
    );
    ::sys::kcall::ipc::__kcall_send(&req)?;
    let bytes_pulled: usize =
        ::sys::kcall::ipc::__kcall_pull(crate::VFS_PUSH_PULL_PID, crate::VFS_PUSH_PULL_TID, out)?;
    let response: Message = ::sys::kcall::ipc::__kcall_recv()?;
    check_status(&response)?;

    // On success vfsd pushes the full payload; a short pull would leave `out` only partially
    // updated, so reject it rather than hand back a corrupt `termios`/`winsize`.
    if bytes_pulled != len {
        ::syslog::warn!(
            "tty_pull(): short pull (fd={fd}, request={request:#x}, expected={len}, \
             pulled={bytes_pulled})"
        );
        return Err(Error::new(ErrorCode::InvalidMessage, "ioctl: terminal payload truncated"));
    }
    Ok(0)
}

/// Stores a terminal attribute payload from `data` into vfsd (a *set* request).
///
/// Sends the metadata request, pushes the `len`-byte payload to vfsd, then receives the status
/// response — mirroring the `write` bulk-transfer protocol.
#[cfg(feature = "standalone")]
fn tty_push(fd: i32, request: c_int, data: &[u8]) -> Result<c_int, Error> {
    use crate::sys::ioctl::message::TtyControlRequest;
    use ::sys::ipc::Message;

    let tid: ::sys::pm::ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;

    let req: Message = TtyControlRequest::build(
        tid,
        fd,
        request,
        data.len() as u32,
        crate::VFS_DESTINATION,
        crate::VFS_MESSAGE_TYPE,
    );
    ::sys::kcall::ipc::__kcall_send(&req)?;
    ::sys::kcall::ipc::__kcall_push(crate::VFS_PUSH_PULL_PID, crate::VFS_PUSH_PULL_TID, data)?;
    let response: Message = ::sys::kcall::ipc::__kcall_recv()?;
    check_status(&response)?;
    Ok(0)
}

/// Maps a non-zero response status to an [`Error`].
#[cfg(feature = "standalone")]
fn check_status(response: &::sys::ipc::Message) -> Result<(), Error> {
    let status: i32 = response.status;
    if status == 0 {
        return Ok(());
    }
    match ErrorCode::try_from(status) {
        Ok(code) => Err(Error::new(code, "ioctl() failed")),
        Err(_) => Err(Error::new(ErrorCode::TryAgain, "ioctl() failed")),
    }
}
