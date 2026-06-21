// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::error::{
    build_error,
    fat32_to_error_code,
};
use ::alloc::vec::Vec;
use ::arch::mem::PAGE_SIZE;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
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
    fcntl::file_status_flags,
    sys_ioctl::{
        Winsize,
        TCGETS,
        TCSETS,
        TIOCGWINSZ,
        TIOCSWINSZ,
    },
    sys_types::c_size_t,
    termios::Termios,
    unistd::{
        STDIN_FILENO,
        STDOUT_FILENO,
    },
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
use ::vfs::{
    fd::{
        ConsoleStream,
        TtyError,
    },
    line_discipline::ConsoleReadOutcome,
};

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

/// Number of raw host-console bytes fetched from the kernel per blocking fetch.
///
/// Reading a small chunk (rather than a single byte) per round-trip lets a burst of buffered or
/// pasted input — and piped input in tests — be cooked in one kernel exchange. Interactive,
/// per-keystroke input still arrives one byte at a time regardless, since the host delivers each
/// keystroke as it is typed. Over-reading is safe: surplus bytes are buffered by the line
/// discipline and served to later reads.
const CONSOLE_RAW_READ_SIZE: usize = 256;

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

/// Handles a `read()` on a console descriptor.
///
/// Cooked input already buffered in the line discipline is served without blocking. When none is
/// available and the descriptor is blocking, this fetches raw input from the host console and feeds
/// it through the line discipline until a readable unit (a canonical line, a raw byte, or EOF)
/// becomes available.
///
/// The host console protocol is synchronous and demand-driven — the host blocks until input arrives
/// and delivers it only in response to an explicit request — and vfsd is single-threaded, so this
/// fetch necessarily blocks the event loop while waiting for the user. Requests from other clients
/// that arrive meanwhile are queued (never lost) and are serviced as soon as the read completes.
/// Crucially, the raw fetch does not consume the kernel's acknowledgement with a nested
/// `__kcall_recv()` (which could dequeue an unrelated request from the shared mailbox); those
/// acknowledgements are drained by the main event loop instead.
pub(crate) fn handle_console_read(
    source_pid: ProcessIdentifier,
    source_tid: ThreadIdentifier,
    fd: i32,
    stream: ConsoleStream,
    count: usize,
) -> Message {
    if stream != ConsoleStream::Stdin {
        let _ = push_to_reader(source_pid, source_tid, &[], "console read");
        return build_error(source_tid, ErrorCode::BadFile);
    }

    let buf_size: usize = count.min(MAX_BULK_TRANSFER_SIZE);
    // Safety: vfsd is single-threaded; no concurrent access to BULK_BUFFER.
    let buf: &mut [u8] = unsafe { &mut BULK_BUFFER[..buf_size] };

    loop {
        match ::vfs::fd::vfs_console_read(fd, buf) {
            Ok(ConsoleReadOutcome::Read(n)) => {
                if let Err(code) = push_to_reader(source_pid, source_tid, &buf[..n], "console read")
                {
                    return build_error(source_tid, code);
                }
                return ReadResponse::build(
                    source_tid,
                    n as i32,
                    [0u8; ReadResponse::BUFFER_SIZE],
                    ProcessIdentifier::VFSD,
                    MessageType::Ipc,
                );
            },
            Ok(ConsoleReadOutcome::Eof) => {
                let _ = push_to_reader(source_pid, source_tid, &[], "console read");
                return ReadResponse::build(
                    source_tid,
                    0,
                    [0u8; ReadResponse::BUFFER_SIZE],
                    ProcessIdentifier::VFSD,
                    MessageType::Ipc,
                );
            },
            Ok(ConsoleReadOutcome::WouldBlock) => {
                if is_nonblocking(fd) {
                    let _ = push_to_reader(source_pid, source_tid, &[], "console read");
                    return build_error(source_tid, ErrorCode::TryAgain);
                }
                if let Err(code) = feed_console_input(fd) {
                    let _ = push_to_reader(source_pid, source_tid, &[], "console read");
                    return build_error(source_tid, code);
                }
            },
            Err(error) => {
                let _ = push_to_reader(source_pid, source_tid, &[], "console read");
                return build_error(source_tid, tty_error_code(error));
            },
        }
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

/// Returns `true` if the open file description for `fd` has `O_NONBLOCK` set.
fn is_nonblocking(fd: i32) -> bool {
    ::vfs::fd::vfs_get_status_flags(fd) & file_status_flags::O_NONBLOCK != 0
}

/// Pushes data to a reader blocked in `__kcall_pull`.
fn push_to_reader(
    pid: ProcessIdentifier,
    tid: ThreadIdentifier,
    data: &[u8],
    context: &'static str,
) -> Result<(), ErrorCode> {
    if let Err(error) = ::sys::kcall::ipc::__kcall_push(pid, tid, data) {
        ::syslog::error!("{context}: push failed (error={:?})", error);
        return Err(ErrorCode::IoErr);
    }
    Ok(())
}

/// Feeds a chunk of raw host-console input through the line discipline.
///
/// Fetches up to [`CONSOLE_RAW_READ_SIZE`] raw bytes from the kernel console, cooks them through the
/// line discipline, and writes back any bytes the discipline wants echoed. A zero-length fetch means
/// end-of-file, which is signalled to the discipline. Echo failures are non-fatal and only logged.
fn feed_console_input(fd: i32) -> Result<(), ErrorCode> {
    let mut raw: [u8; CONSOLE_RAW_READ_SIZE] = [0u8; CONSOLE_RAW_READ_SIZE];
    let n: usize = kernel_read_stdin(&mut raw).map_err(|error| error.code)?;
    if n == 0 {
        return ::vfs::fd::vfs_console_push_eof(fd).map_err(tty_error_code);
    }

    let echo: Vec<u8> = ::vfs::fd::vfs_console_push_input(fd, &raw[..n]).map_err(tty_error_code)?;
    if !echo.is_empty() {
        if let Err(error) = kernel_write_console(STDOUT_FILENO, &echo) {
            ::syslog::warn!("console read: failed to echo input (error={:?})", error);
        }
    }

    Ok(())
}

/// Reads raw input from the kernel console stdin stream.
///
/// Sends a `ReadRequest` to the kernel console and pulls the delivered bytes. The pulled byte count
/// is authoritative: the host always completes the pull (with `0` bytes on end-of-file). The
/// kernel's matching `ReadResponse` acknowledgement is intentionally left in vfsd's mailbox for the
/// main event loop to discard, rather than consumed here with a nested `__kcall_recv()` that could
/// instead dequeue an unrelated guest request (the mailbox is shared by the whole process).
fn kernel_read_stdin(buf: &mut [u8]) -> Result<usize, Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;
    let request: Message = ReadRequest::build(
        tid,
        STDIN_FILENO,
        buf.len() as c_size_t,
        ProcessIdentifier::KERNEL,
        MessageType::Ikc,
    );
    ::sys::kcall::ipc::__kcall_send(&request)?;
    let bytes_pulled: usize =
        ::sys::kcall::ipc::__kcall_pull(ProcessIdentifier::KERNEL, ThreadIdentifier::KERNEL, buf)?;
    Ok(bytes_pulled)
}

/// Writes bytes to the kernel console output stream (used to echo cooked input).
///
/// Like [`kernel_read_stdin`], the kernel's `WriteResponse` acknowledgement is left in vfsd's
/// mailbox for the main event loop to discard rather than consumed with a nested `__kcall_recv()`,
/// which could otherwise dequeue an unrelated guest request from the shared mailbox.
fn kernel_write_console(fd: i32, buf: &[u8]) -> Result<(), Error> {
    if buf.is_empty() {
        return Ok(());
    }

    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;
    let empty_buf: [u8; WriteRequest::BUFFER_SIZE] = [0u8; WriteRequest::BUFFER_SIZE];
    let request: Message = WriteRequest::build(
        tid,
        fd,
        buf.len() as c_size_t,
        empty_buf,
        ProcessIdentifier::KERNEL,
        MessageType::Ikc,
    );
    ::sys::kcall::ipc::__kcall_send(&request)?;
    ::sys::kcall::ipc::__kcall_push(ProcessIdentifier::KERNEL, ThreadIdentifier::KERNEL, buf)?;
    Ok(())
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
