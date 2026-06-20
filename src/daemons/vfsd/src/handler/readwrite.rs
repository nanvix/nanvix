// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::error::{
    build_error,
    fat32_to_error_code,
};
use ::arch::mem::PAGE_SIZE;
use ::sys::{
    error::ErrorCode,
    ipc::{
        Message,
        MessageType,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
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
use ::syscall::{
    sys::ioctl::message::{
        TtyControlRequest,
        TtyControlResponse,
    },
    unistd::message::{
        PartialReadRequest,
        PartialReadResponse,
        PartialWriteRequest,
        PartialWriteResponse,
        ReadRequest,
        ReadResponse,
        WriteRequest,
        WriteResponse,
    },
    SystemCallMessage,
};
use ::vfs::fd::TtyError;

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum number of bytes transferred in a single read/write bulk operation.
/// Must be at least as large as the page-aligned chunk size used by the syscall layer.
const MAX_BULK_TRANSFER_SIZE: usize = PAGE_SIZE;

/// Static buffer used for bulk read/write data transfers.
/// Safety: vfsd processes one request at a time (single-threaded message loop),
/// so there is no concurrent access to this buffer.
static mut BULK_BUFFER: [u8; MAX_BULK_TRANSFER_SIZE] = [0u8; MAX_BULK_TRANSFER_SIZE];

//==================================================================================================
// Read/Write Handlers (with push/pull bulk data transfer)
//==================================================================================================

pub(crate) fn handle_read(
    source_pid: ProcessIdentifier,
    source_tid: ThreadIdentifier,
    msg: SystemCallMessage,
) -> Message {
    let req: ReadRequest = ReadRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;
    let count: usize = req.count as usize;

    // Cap the read to the maximum bulk transfer size.
    let buf_size: usize = if count > MAX_BULK_TRANSFER_SIZE {
        MAX_BULK_TRANSFER_SIZE
    } else {
        count
    };

    // Safety: vfsd is single-threaded; no concurrent access to BULK_BUFFER.
    let buf: &mut [u8] = unsafe { &mut BULK_BUFFER[..buf_size] };

    match ::vfs::fd::vfs_read(fd, buf) {
        Ok(n) => {
            let n: usize = n as usize;

            // Push the data to the caller.
            if let Err(e) = ::sys::kcall::ipc::__kcall_push(source_pid, source_tid, &buf[..n]) {
                ::syslog::error!("handle_read(): push failed (error={:?})", e);
                return build_error(source_tid, ErrorCode::IoErr);
            }

            ReadResponse::build(
                source_tid,
                n as i32,
                [0u8; ReadResponse::BUFFER_SIZE],
                ProcessIdentifier::VFSD,
                MessageType::Ipc,
            )
        },
        Err(e) => {
            // The client is blocked on __kcall_pull — push an empty buffer to unblock it
            // before sending the error response, otherwise the client deadlocks.
            if let Err(push_err) = ::sys::kcall::ipc::__kcall_push(source_pid, source_tid, &[]) {
                ::syslog::error!("handle_read(): unblock push failed (error={:?})", push_err);
            }
            build_error(source_tid, fat32_to_error_code(&e))
        },
    }
}

pub(crate) fn handle_write(
    source_pid: ProcessIdentifier,
    source_tid: ThreadIdentifier,
    msg: SystemCallMessage,
) -> Message {
    let req: WriteRequest = WriteRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;
    let count: usize = req.count as usize;

    // Cap to the maximum bulk transfer size.
    let buf_size: usize = if count > MAX_BULK_TRANSFER_SIZE {
        MAX_BULK_TRANSFER_SIZE
    } else {
        count
    };

    // Safety: vfsd is single-threaded; no concurrent access to BULK_BUFFER.
    let buf: &mut [u8] = unsafe { &mut BULK_BUFFER[..buf_size] };

    // Pull the data from the caller.
    match ::sys::kcall::ipc::__kcall_pull(source_pid, source_tid, buf) {
        Ok(pulled) => {
            let write_len: usize = if pulled < count { pulled } else { count };
            match ::vfs::fd::vfs_write(fd, &buf[..write_len]) {
                Ok(n) => WriteResponse::build(
                    source_tid,
                    n as i32,
                    ProcessIdentifier::VFSD,
                    MessageType::Ipc,
                ),
                Err(e) => build_error(source_tid, fat32_to_error_code(&e)),
            }
        },
        Err(e) => {
            ::syslog::error!("handle_write(): pull failed (error={:?})", e);
            build_error(source_tid, ErrorCode::IoErr)
        },
    }
}

//==================================================================================================
// Partial Read/Write Handlers (inline data in message payload)
//==================================================================================================

pub(crate) fn handle_pread(source: ThreadIdentifier, msg: SystemCallMessage) -> Message {
    let req: PartialReadRequest = PartialReadRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;
    let count: usize = req.count as usize;
    let offset = req.offset;

    let max_inline: usize = PartialReadResponse::BUFFER_SIZE;
    let read_count: usize = if count > max_inline {
        max_inline
    } else {
        count
    };
    let mut buf = [0u8; PartialReadResponse::BUFFER_SIZE];

    match ::vfs::fd::vfs_pread(fd, &mut buf[..read_count], offset) {
        Ok(n) => PartialReadResponse::build(
            source,
            n as i32,
            buf,
            ProcessIdentifier::VFSD,
            MessageType::Ipc,
        ),
        Err(e) => build_error(source, fat32_to_error_code(&e)),
    }
}

pub(crate) fn handle_pwrite(source: ThreadIdentifier, msg: SystemCallMessage) -> Message {
    let req: PartialWriteRequest = PartialWriteRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;
    let count: usize = req.count as usize;
    let offset = req.offset;

    let max_inline: usize = PartialWriteRequest::BUFFER_SIZE;
    let write_count: usize = if count > max_inline {
        max_inline
    } else {
        count
    };

    match ::vfs::fd::vfs_pwrite(fd, &req.buffer[..write_count], offset) {
        Ok(n) => {
            PartialWriteResponse::build(source, n as i32, ProcessIdentifier::VFSD, MessageType::Ipc)
        },
        Err(e) => build_error(source, fat32_to_error_code(&e)),
    }
}

//==================================================================================================
// Terminal-Control Handler (push/pull bulk transfer of termios/winsize)
//==================================================================================================

/// Maps a terminal-control error to the matching POSIX error code.
fn tty_error_code(error: TtyError) -> ErrorCode {
    match error {
        // An unknown descriptor is a bad file descriptor.
        TtyError::BadFd => ErrorCode::BadFile,
        // A valid non-terminal descriptor is not a typewriter.
        TtyError::NotTty => ErrorCode::NotTerminal,
    }
}

/// Handles a terminal-control `ioctl` (`TCGETS`/`TCSETS`/`TIOCGWINSZ`/`TIOCSWINSZ`).
///
/// The `termios`/`winsize` payload is transferred out of band via the push/pull rendezvous, the same
/// way `read`/`write` carry their bulk data: a *get* fetches the attributes from the shared console
/// terminal and pushes them to the caller, while a *set* pulls the attributes from the caller and
/// stores them. As with `read`/`write`, an error path still completes the rendezvous (an empty push
/// to release a *get* caller blocked in `__kcall_pull`) before the error response is sent, so the
/// caller never deadlocks.
pub(crate) fn handle_tty_control(
    source_pid: ProcessIdentifier,
    source_tid: ThreadIdentifier,
    msg: SystemCallMessage,
) -> Message {
    let req: TtyControlRequest = TtyControlRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;
    let request: i32 = req.request;
    let len: usize = req.len as usize;

    match request {
        // Set requests pull the payload from the caller, then store it on the shared terminal.
        TCSETS | TIOCSWINSZ => {
            // The console payload never exceeds a `termios`; cap the pull at its size.
            let mut buf: [u8; Termios::SIZE] = [0u8; Termios::SIZE];
            let pull_len: usize = if len > buf.len() { buf.len() } else { len };
            match ::sys::kcall::ipc::__kcall_pull(source_pid, source_tid, &mut buf[..pull_len]) {
                Ok(pulled) => {
                    let outcome: Result<(), ErrorCode> = if request == TCSETS {
                        let termios: Termios = Termios::from_bytes(&buf[..pulled]);
                        ::vfs::fd::vfs_tty_set_termios(fd, termios).map_err(tty_error_code)
                    } else {
                        let winsize: Winsize = Winsize::from_bytes(&buf[..pulled]);
                        ::vfs::fd::vfs_tty_set_winsize(fd, winsize).map_err(tty_error_code)
                    };
                    match outcome {
                        Ok(()) => TtyControlResponse::build(
                            source_tid,
                            0,
                            ProcessIdentifier::VFSD,
                            MessageType::Ipc,
                        ),
                        Err(code) => build_error(source_tid, code),
                    }
                },
                Err(e) => {
                    ::syslog::error!("handle_tty_control(): pull failed (error={:?})", e);
                    build_error(source_tid, ErrorCode::IoErr)
                },
            }
        },
        // Get requests fetch the payload from the shared terminal, then push it to the caller.
        TCGETS | TIOCGWINSZ => {
            let mut buf: [u8; Termios::SIZE] = [0u8; Termios::SIZE];
            let payload: Result<usize, ErrorCode> = if request == TCGETS {
                ::vfs::fd::vfs_tty_get_termios(fd)
                    .map(|termios| copy_into(&mut buf, &termios.to_bytes()))
                    .map_err(tty_error_code)
            } else {
                ::vfs::fd::vfs_tty_get_winsize(fd)
                    .map(|winsize| copy_into(&mut buf, winsize.as_bytes()))
                    .map_err(tty_error_code)
            };
            match payload {
                Ok(n) => {
                    if let Err(e) =
                        ::sys::kcall::ipc::__kcall_push(source_pid, source_tid, &buf[..n])
                    {
                        ::syslog::error!("handle_tty_control(): push failed (error={:?})", e);
                        return build_error(source_tid, ErrorCode::IoErr);
                    }
                    TtyControlResponse::build(
                        source_tid,
                        0,
                        ProcessIdentifier::VFSD,
                        MessageType::Ipc,
                    )
                },
                Err(code) => {
                    // The caller is blocked in `__kcall_pull`; release it with an empty push before
                    // reporting the error, otherwise it deadlocks.
                    if let Err(push_err) =
                        ::sys::kcall::ipc::__kcall_push(source_pid, source_tid, &[])
                    {
                        ::syslog::error!(
                            "handle_tty_control(): unblock push failed (error={:?})",
                            push_err
                        );
                    }
                    build_error(source_tid, code)
                },
            }
        },
        // No other request reaches vfsd: the client forwards only terminal-control requests.
        other => {
            ::syslog::warn!("handle_tty_control(): unsupported request {other:#x}");
            build_error(source_tid, ErrorCode::NotTerminal)
        },
    }
}

/// Copies `src` into the front of `dst` and returns the number of bytes copied.
fn copy_into(dst: &mut [u8], src: &[u8]) -> usize {
    let n: usize = if src.len() < dst.len() {
        src.len()
    } else {
        dst.len()
    };
    dst[..n].copy_from_slice(&src[..n]);
    n
}
