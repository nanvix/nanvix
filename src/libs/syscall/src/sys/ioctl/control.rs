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

use crate::poll::input_message::PipeOperation;
use ::core::time::Duration;
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::ffi::{
    c_int,
    c_void,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Maximum time a terminal setter waits for VFSD's bulk pull.
const BULK_PUSH_TIMEOUT: Duration = Duration::from_secs(1);

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
    ioctl_vfsd(fd, request, arg)
}

/// Routes a terminal-control request to vfsd, or ignores a non-terminal request.
fn ioctl_vfsd(fd: c_int, request: c_int, arg: *mut c_void) -> Result<c_int, Error> {
    use ::sysapi::{
        sys_ioctl::{
            Winsize,
            TCGETS,
            TCSETS,
            TIOCGPGRP,
            TIOCGWINSZ,
            TIOCSPGRP,
            TIOCSWINSZ,
        },
        termios::Termios,
    };

    // Foreground-process-group control is job control, owned by the process manager daemon rather
    // than the vfsd console backend. Route these requests there.
    if request == TIOCGPGRP || request == TIOCSPGRP {
        return ioctl_pgrp(fd, request, arg);
    }

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

/// Answers a foreground-process-group ioctl (`TIOCGPGRP`/`TIOCSPGRP`) on a terminal descriptor by
/// consulting the process manager daemon, which owns the controlling terminal's foreground group.
///
/// These two requests back `tcgetpgrp()`/`tcsetpgrp()`; the argument is a `pid_t` rather than a
/// `termios`/`winsize` payload, so they are answered here directly instead of through the vfsd
/// push/pull transfer used by the other terminal requests.
fn ioctl_pgrp(fd: c_int, request: c_int, arg: *mut c_void) -> Result<c_int, Error> {
    use ::sys::pm::ProcessIdentifier;
    use ::sysapi::{
        sys_ioctl::TIOCGPGRP,
        sys_types::pid_t,
    };

    // The argument is a `pid_t` the caller owns; it must be a valid pointer.
    if arg.is_null() {
        ::syslog::warn!("ioctl(): null argument pointer (fd={fd}, request={request:#x})");
        return Err(Error::new(ErrorCode::BadAddress, "ioctl: null argument pointer"));
    }

    // Only a console descriptor is a terminal; a non-console fd is ENOTTY, an unknown fd is EBADF.
    let _tty_fd: i32 = crate::fdtable::resolve_tty(fd, "ioctl")?;

    if request == TIOCGPGRP {
        let pgrp: ProcessIdentifier = ::proc::tcgetpgrp()?;
        // Safety: `arg` is non-null and points to a `pid_t` the caller owns.
        unsafe {
            *(arg as *mut pid_t) = i32::from(pgrp);
        }
        Ok(0)
    } else {
        // Safety: `arg` is non-null and points to a `pid_t` the caller owns.
        let pgrp: pid_t = unsafe { *(arg as *const pid_t) };
        ::proc::tcsetpgrp(ProcessIdentifier::from(pgrp))?;
        Ok(0)
    }
}

/// Fetches a terminal attribute payload from vfsd into `out` (a *get* request).
///
/// Sends the metadata request, pulls the `len`-byte payload that vfsd pushes, then receives the
/// status response — mirroring the `read` bulk-transfer protocol.
fn tty_pull(fd: i32, request: c_int, out: *mut u8, len: usize) -> Result<c_int, Error> {
    use crate::sys::ioctl::message::TtyControlRequest;
    use ::sys::ipc::{
        Message,
        RequestToken,
    };

    let tid: ::sys::pm::ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;
    let signal_mask: crate::rpc::SignalMaskGuard = crate::rpc::SignalMaskGuard::block_all()?;

    // Safety: `out` is non-null (checked by the caller) and points to a `len`-byte object the caller
    // owns; vfsd pushes exactly `len` bytes into it.
    let out: &mut [u8] = unsafe { ::core::slice::from_raw_parts_mut(out, len) };

    let mut req: Message = TtyControlRequest::build(
        tid,
        fd,
        request,
        len as u32,
        crate::VFS_DESTINATION,
        crate::VFS_MESSAGE_TYPE,
    );
    let token: RequestToken = crate::rpc::send_request(&mut req)?;
    let pull_result: Result<usize, Error> =
        ::sys::kcall::ipc::__kcall_pull_tagged_restoring_signals(
            crate::VFS_PUSH_PULL_PID,
            crate::VFS_PUSH_PULL_TID,
            out,
            token.identifier(),
            signal_mask.previous(),
        );
    drop(signal_mask);
    let bytes_pulled: usize = match pull_result {
        Ok(bytes_pulled) => bytes_pulled,
        Err(error) if error.code == ErrorCode::Interrupted => {
            if crate::unistd::syscall::cancel_pipe_operation(
                tid,
                fd,
                PipeOperation::Read,
                token.identifier(),
            )?
            .is_some()
            {
                return Err(error);
            }
            let _response: Message = crate::rpc::recv_response(&token)?;
            return Err(error);
        },
        Err(error) => return Err(error),
    };
    let response: Message = crate::rpc::recv_response(&token)?;
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
fn tty_push(fd: i32, request: c_int, data: &[u8]) -> Result<c_int, Error> {
    use crate::sys::ioctl::message::TtyControlRequest;
    use ::sys::ipc::{
        Message,
        RequestToken,
    };

    let tid: ::sys::pm::ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;
    let signal_mask: crate::rpc::SignalMaskGuard = crate::rpc::SignalMaskGuard::block_all()?;

    let mut req: Message = TtyControlRequest::build(
        tid,
        fd,
        request,
        data.len() as u32,
        crate::VFS_DESTINATION,
        crate::VFS_MESSAGE_TYPE,
    );
    let token: RequestToken = crate::rpc::send_request(&mut req)?;
    let push_result: Result<(), Error> =
        ::sys::kcall::ipc::__kcall_push_tagged_restoring_signals_timed(
            crate::VFS_PUSH_PULL_PID,
            crate::VFS_PUSH_PULL_TID,
            data,
            token.identifier(),
            signal_mask.previous(),
            BULK_PUSH_TIMEOUT,
        );
    drop(signal_mask);
    match push_result {
        Ok(()) => {},
        Err(error) if error.code == ErrorCode::OperationTimedOut => return Err(error),
        Err(error) if error.code == ErrorCode::Interrupted => {
            let _ = crate::unistd::syscall::cancel_pipe_operation(
                tid,
                fd,
                PipeOperation::Write,
                token.identifier(),
            )?;
            return Err(error);
        },
        Err(error) => return Err(error),
    }
    let response: Message = crate::rpc::recv_response(&token)?;
    check_status(&response)?;
    Ok(0)
}

/// Maps a non-zero response status to an [`Error`].
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
