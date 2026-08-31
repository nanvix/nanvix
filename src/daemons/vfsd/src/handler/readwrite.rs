// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    console_wait::{
        BlockedConsoleReader,
        ConsoleWaitTable,
    },
    error::{
        build_error,
        fat32_to_error_code,
        ResponseContext,
    },
};
use ::alloc::{
    boxed::Box,
    vec::Vec,
};
use ::arch::mem::PAGE_SIZE;
use ::core::time::Duration;
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
        SIGINT,
        SIGQUIT,
        SIGTSTP,
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
    unistd::STDOUT_FILENO,
};
use ::syscall::{
    poll::input_message::{
        ConsoleReadRetry,
        PollInputRequest,
    },
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
    fd::TtyError,
    line_discipline::{
        ConsoleReadOutcome,
        TerminalSignal,
    },
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

/// Immediate host-console input snapshot.
enum ConsoleInputSnapshot {
    Empty,
    Eof,
    Data(Box<[u8]>),
}

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
/// Cooked input already buffered in the line discipline is served without blocking. A blocking read
/// with no cooked input is parked in `console_wait` and revived by an input-availability
/// notification; VFSD therefore never blocks its event loop waiting for a host keystroke.
pub(crate) fn handle_console_read(
    response_context: ResponseContext,
    fd: i32,
    readable: bool,
    count: usize,
    console_wait: &mut ConsoleWaitTable,
) -> Option<Message> {
    let source_pid: ProcessIdentifier = response_context.source_pid();
    let source_tid: ThreadIdentifier = response_context.source_tid();
    if !readable {
        console_wait.park(BlockedConsoleReader {
            response_context,
            source_pid,
            source_tid,
            fd,
            count,
            error: Some(ErrorCode::BadFile),
        });
        wake_console_readers(console_wait);
        service_pending_console_input(console_wait);
        return None;
    }

    if count > 0 {
        // Report the console read to the process manager daemon so that it can raise `SIGTTIN` when
        // the reader is in a background process group. The notification is fire-and-forget: job
        // control is the daemon's concern, and a process with no foreground group established (the
        // common case) is never signalled, so this is a no-op for ordinary readers.
        notify_terminal_access(source_pid, false);
    }

    console_wait.park(BlockedConsoleReader {
        response_context,
        source_pid,
        source_tid,
        fd,
        count,
        error: None,
    });
    if !service_pending_console_input(console_wait) {
        wake_console_readers(console_wait);
    }
    None
}

/// Handles a write to an opened terminal device.
pub(crate) fn handle_terminal_write(
    source_pid: ProcessIdentifier,
    source_tid: ThreadIdentifier,
    msg: SystemCallMessage,
    writable: bool,
) -> Message {
    let req: WriteRequest = WriteRequest::from_bytes(msg.payload);
    let count: usize = req.count as usize;
    let buf_size: usize = count.min(MAX_BULK_TRANSFER_SIZE);
    // Safety: vfsd is single-threaded; no concurrent access to BULK_BUFFER.
    let buf: &mut [u8] = unsafe { &mut BULK_BUFFER[..buf_size] };

    match ::sys::kcall::ipc::__kcall_pull(source_pid, source_tid, buf) {
        Ok(pulled) => {
            if !writable {
                return build_error(source_tid, ErrorCode::BadFile);
            }
            let write_len: usize = pulled.min(count);
            if write_len > 0 {
                notify_terminal_access(source_pid, true);
                if let Err(error) = kernel_write_console(STDOUT_FILENO, &buf[..write_len]) {
                    ::syslog::error!("terminal write failed (error={:?})", error);
                    return build_error(source_tid, ErrorCode::IoErr);
                }
            }
            WriteResponse::build(
                source_tid,
                write_len as i32,
                ProcessIdentifier::VFSD,
                MessageType::Ipc,
            )
        },
        Err(error) => {
            ::syslog::error!("terminal write pull failed (error={:?})", error);
            build_error(source_tid, ErrorCode::IoErr)
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

/// Returns `true` if the open file description for `fd` has `O_NONBLOCK` set.
fn is_nonblocking(fd: i32) -> bool {
    ::vfs::fd::vfs_get_status_flags(fd) & file_status_flags::O_NONBLOCK != 0
}

/// Tries to feed currently buffered host-console input without waiting for a keystroke.
pub(crate) fn try_feed_console_input(fd: i32) -> Result<bool, ErrorCode> {
    match console_input_snapshot()? {
        ConsoleInputSnapshot::Empty => Ok(false),
        ConsoleInputSnapshot::Eof => {
            ::vfs::fd::vfs_console_push_eof(fd).map_err(tty_error_code)?;
            Ok(false)
        },
        ConsoleInputSnapshot::Data(raw) => {
            process_console_input(fd, &raw)?;
            Ok(true)
        },
    }
}

/// Fetches one immediate host-console input snapshot.
fn console_input_snapshot() -> Result<ConsoleInputSnapshot, ErrorCode> {
    let mut response: [u8; CONSOLE_RAW_READ_SIZE + 1] = [0u8; CONSOLE_RAW_READ_SIZE + 1];
    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid().map_err(|error| error.code)?;
    let request: Message = PollInputRequest::build(tid, CONSOLE_RAW_READ_SIZE as u32);
    ::sys::kcall::ipc::__kcall_send(&request).map_err(|error| error.code)?;
    let n: usize = ::sys::kcall::ipc::__kcall_pull(
        ProcessIdentifier::KERNEL,
        ThreadIdentifier::KERNEL,
        &mut response,
    )
    .map_err(|error| error.code)?;
    if n == 0 {
        return Err(ErrorCode::InvalidMessage);
    }

    match response[0] {
        PollInputRequest::STATUS_EMPTY => Ok(ConsoleInputSnapshot::Empty),
        PollInputRequest::STATUS_EOF => Ok(ConsoleInputSnapshot::Eof),
        PollInputRequest::STATUS_DATA if n > 1 => {
            let raw: Box<[u8]> = Box::from(&response[1..n]);
            Ok(ConsoleInputSnapshot::Data(raw))
        },
        _ => Err(ErrorCode::InvalidMessage),
    }
}

/// Retains one host-input availability token and services it when a reader needs input.
pub(crate) fn handle_console_input_available(console_wait: &mut ConsoleWaitTable) {
    console_wait.mark_input_available();
    service_pending_console_input(console_wait);
}

/// Fetches one retained host-input snapshot for the front reader.
pub(crate) fn service_pending_console_input(console_wait: &mut ConsoleWaitTable) -> bool {
    if !console_wait.front_needs_input() || !console_wait.take_input_available() {
        return false;
    }

    match console_input_snapshot() {
        Ok(ConsoleInputSnapshot::Data(raw)) => process_console_device_input(&raw),
        Ok(ConsoleInputSnapshot::Eof) => {
            ::vfs::fd::vfs_console_device_push_eof();
        },
        Ok(ConsoleInputSnapshot::Empty) => {},
        Err(error) => {
            ::syslog::warn!("console input notification failed (error={:?})", error);
        },
    }
    wake_console_readers(console_wait);
    true
}

/// Revives queued console readers in FIFO order while cooked data or EOF is available.
fn wake_console_readers(console_wait: &mut ConsoleWaitTable) {
    while let Some(reader) = console_wait.front() {
        if let Some(error) = reader.error {
            match ::sys::kcall::ipc::__kcall_push_timed(
                reader.source_pid,
                reader.source_tid,
                &[],
                Some(Duration::ZERO),
            ) {
                Ok(()) => {
                    console_wait.pop();
                    reader
                        .response_context
                        .send(&build_error(reader.source_tid, error));
                    continue;
                },
                Err(push_error) if push_error.code == ErrorCode::OperationTimedOut => {
                    schedule_console_read_retry(console_wait);
                    break;
                },
                Err(_) => {
                    console_wait.pop();
                    continue;
                },
            }
        }

        ::vfs::fd::set_current_process(reader.source_pid);
        let buf_size: usize = reader.count.min(MAX_BULK_TRANSFER_SIZE);
        // Safety: VFSD is single-threaded and releases this borrow before processing another IPC.
        let buf: &mut [u8] = unsafe { &mut BULK_BUFFER[..buf_size] };
        match ::vfs::fd::vfs_console_peek(reader.fd, buf) {
            Ok(ConsoleReadOutcome::Read(n)) => {
                match ::sys::kcall::ipc::__kcall_push_timed(
                    reader.source_pid,
                    reader.source_tid,
                    &buf[..n],
                    Some(Duration::ZERO),
                ) {
                    Ok(()) => {},
                    Err(error) if error.code == ErrorCode::OperationTimedOut => {
                        schedule_console_read_retry(console_wait);
                        break;
                    },
                    Err(_) => {
                        console_wait.pop();
                        continue;
                    },
                }

                let consumed: Result<ConsoleReadOutcome, TtyError> =
                    ::vfs::fd::vfs_console_read(reader.fd, buf);
                console_wait.pop();
                match consumed {
                    Ok(ConsoleReadOutcome::Read(consumed)) if consumed == n => {
                        reader.response_context.send(&ReadResponse::build(
                            reader.source_tid,
                            n as i32,
                            [0u8; ReadResponse::BUFFER_SIZE],
                            ProcessIdentifier::VFSD,
                            MessageType::Ipc,
                        ));
                    },
                    _ => reader
                        .response_context
                        .send(&build_error(reader.source_tid, ErrorCode::InvalidMessage)),
                }
            },
            Ok(ConsoleReadOutcome::Eof) => {
                match ::sys::kcall::ipc::__kcall_push_timed(
                    reader.source_pid,
                    reader.source_tid,
                    &[],
                    Some(Duration::ZERO),
                ) {
                    Ok(()) => {},
                    Err(error) if error.code == ErrorCode::OperationTimedOut => {
                        schedule_console_read_retry(console_wait);
                        break;
                    },
                    Err(_) => {
                        console_wait.pop();
                        continue;
                    },
                }

                let consumed: Result<ConsoleReadOutcome, TtyError> =
                    ::vfs::fd::vfs_console_read(reader.fd, buf);
                console_wait.pop();
                match consumed {
                    Ok(ConsoleReadOutcome::Eof) => {
                        reader.response_context.send(&ReadResponse::build(
                            reader.source_tid,
                            0,
                            [0u8; ReadResponse::BUFFER_SIZE],
                            ProcessIdentifier::VFSD,
                            MessageType::Ipc,
                        ))
                    },
                    _ => reader
                        .response_context
                        .send(&build_error(reader.source_tid, ErrorCode::InvalidMessage)),
                }
            },
            Ok(ConsoleReadOutcome::WouldBlock) if is_nonblocking(reader.fd) => {
                match ::sys::kcall::ipc::__kcall_push_timed(
                    reader.source_pid,
                    reader.source_tid,
                    &[],
                    Some(Duration::ZERO),
                ) {
                    Ok(()) => {
                        console_wait.pop();
                        reader
                            .response_context
                            .send(&build_error(reader.source_tid, ErrorCode::TryAgain));
                    },
                    Err(error) if error.code == ErrorCode::OperationTimedOut => {
                        schedule_console_read_retry(console_wait);
                        break;
                    },
                    Err(_) => {
                        console_wait.pop();
                    },
                }
            },
            Ok(ConsoleReadOutcome::WouldBlock) => break,
            Err(error) => {
                match ::sys::kcall::ipc::__kcall_push_timed(
                    reader.source_pid,
                    reader.source_tid,
                    &[],
                    Some(Duration::ZERO),
                ) {
                    Ok(()) => {
                        console_wait.pop();
                        reader
                            .response_context
                            .send(&build_error(reader.source_tid, tty_error_code(error)));
                    },
                    Err(push_error) if push_error.code == ErrorCode::OperationTimedOut => {
                        schedule_console_read_retry(console_wait);
                        break;
                    },
                    Err(_) => {
                        console_wait.pop();
                    },
                }
            },
        }
    }
}

/// Attempts delivery to parked console readers without consuming a queued retry marker.
pub(crate) fn service_console_readers(console_wait: &mut ConsoleWaitTable) {
    wake_console_readers(console_wait);
}

/// Yields to let the reader register its pull, then retries through VFSD's event loop.
fn schedule_console_read_retry(console_wait: &mut ConsoleWaitTable) {
    if !console_wait.schedule_read_retry() {
        return;
    }
    if let Err(error) = ::sys::kcall::sched::__kcall_sched_yield() {
        ::syslog::warn!("console read: failed to yield before retry (error={:?})", error);
    }
    let tid: ThreadIdentifier = match ::sys::kcall::pm::__kcall_gettid() {
        Ok(tid) => tid,
        Err(error) => {
            console_wait.consume_read_retry();
            ::syslog::error!("console read: failed to get VFSD tid (error={:?})", error);
            return;
        },
    };
    let retry: Message = ConsoleReadRetry::build(tid);
    if let Err(error) = ::sys::kcall::ipc::__kcall_send(&retry) {
        console_wait.consume_read_retry();
        ::syslog::error!("console read: failed to schedule retry (error={:?})", error);
    }
}

/// Retries delivery to parked console readers after they had time to register their pulls.
pub(crate) fn retry_console_readers(console_wait: &mut ConsoleWaitTable) {
    console_wait.consume_read_retry();
    service_console_readers(console_wait);
    service_pending_console_input(console_wait);
}

/// Cooks raw console bytes and handles echo and terminal-generated signals.
fn process_console_input(fd: i32, raw: &[u8]) -> Result<(), ErrorCode> {
    let echo: Vec<u8> = ::vfs::fd::vfs_console_push_input(fd, raw).map_err(tty_error_code)?;
    if !echo.is_empty() {
        if let Err(error) = kernel_write_console(STDOUT_FILENO, &echo) {
            ::syslog::warn!("console read: failed to echo input (error={:?})", error);
        }
    }

    // Forward any terminal-generated signals (`^C`/`^\`/`^Z`) recognized by the line discipline to
    // the process manager daemon, which owns job control and delivers them to the foreground group.
    forward_console_signals(fd);

    Ok(())
}

/// Cooks raw bytes through the persistent console device and forwards echo and signals.
fn process_console_device_input(raw: &[u8]) {
    let echo: Vec<u8> = ::vfs::fd::vfs_console_device_push_input(raw);
    if !echo.is_empty() {
        if let Err(error) = kernel_write_console(STDOUT_FILENO, &echo) {
            ::syslog::warn!("console input: failed to echo input (error={:?})", error);
        }
    }
    forward_console_device_signals();
}

/// Notifies the process manager daemon that `pid` accessed the console, so it can raise `SIGTTIN`
/// (read) or `SIGTTOU` (write) when `pid` is in a background process group.
///
/// The notification is fire-and-forget for the same reason as [`forward_console_signals`]: the
/// single-threaded daemon must not block its event loop on the process manager daemon. Job-control
/// policy (whether the access is actually from a background group) lives entirely in that daemon.
fn notify_terminal_access(pid: ProcessIdentifier, write: bool) {
    match ::proc::terminal_access_request(ProcessIdentifier::VFSD, pid, write) {
        Ok(message) => {
            if let Err(error) = ::sys::kcall::ipc::__kcall_send(&message) {
                ::syslog::warn!(
                    "console read: failed to notify terminal access (pid={:?}, error={:?})",
                    pid,
                    error
                );
            }
        },
        Err(error) => {
            ::syslog::warn!(
                "console read: failed to build terminal-access notification (error={:?})",
                error
            );
        },
    }
}

/// Forwards the terminal-generated signals the line discipline recognized to the process manager
/// daemon, which delivers them to the controlling terminal's foreground process group.
///
/// Each line-discipline [`TerminalSignal`] maps to its POSIX signal and is sent as a fire-and-forget
/// notification, so the single-threaded daemon never blocks its event loop waiting on the process
/// manager daemon. Failures are logged and otherwise ignored: a dropped terminal signal must not
/// derail the console read that produced it.
fn forward_console_signals(fd: i32) {
    let signals: Vec<TerminalSignal> = match ::vfs::fd::vfs_console_take_signals(fd) {
        Ok(signals) => signals,
        Err(error) => {
            ::syslog::warn!("console read: failed to drain signals (error={:?})", error);
            return;
        },
    };

    for signal in signals {
        let signum: i32 = match signal {
            TerminalSignal::Interrupt => SIGINT as i32,
            TerminalSignal::Quit => SIGQUIT as i32,
            TerminalSignal::Suspend => SIGTSTP as i32,
        };
        match ::proc::terminal_signal_request(ProcessIdentifier::VFSD, signum) {
            Ok(message) => {
                if let Err(error) = ::sys::kcall::ipc::__kcall_send(&message) {
                    ::syslog::warn!(
                        "console read: failed to forward terminal signal (signum={}, error={:?})",
                        signum,
                        error
                    );
                }
            },
            Err(error) => {
                ::syslog::warn!(
                    "console read: failed to build terminal-signal notification (error={:?})",
                    error
                );
            },
        }
    }
}

/// Forwards terminal-generated signals drained directly from the shared console device.
fn forward_console_device_signals() {
    for signal in ::vfs::fd::vfs_console_device_take_signals() {
        let signum: i32 = match signal {
            TerminalSignal::Interrupt => SIGINT as i32,
            TerminalSignal::Quit => SIGQUIT as i32,
            TerminalSignal::Suspend => SIGTSTP as i32,
        };
        match ::proc::terminal_signal_request(ProcessIdentifier::VFSD, signum) {
            Ok(message) => {
                if let Err(error) = ::sys::kcall::ipc::__kcall_send(&message) {
                    ::syslog::warn!(
                        "console input: failed to forward terminal signal (signum={}, error={:?})",
                        signum,
                        error
                    );
                }
            },
            Err(error) => {
                ::syslog::warn!(
                    "console input: failed to build terminal-signal notification (error={:?})",
                    error
                );
            },
        }
    }
}

/// Writes bytes to the kernel console output stream (used to echo cooked input).
///
/// The kernel's `WriteResponse` acknowledgement is left in VFSD's mailbox for the main event loop
/// to discard rather than consumed with a nested `__kcall_recv()`, which could otherwise dequeue an
/// unrelated guest request from the shared mailbox.
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
    console_wait: &mut ConsoleWaitTable,
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
                        Ok(()) => {
                            if request == TCSETS {
                                wake_console_readers(console_wait);
                            }
                            TtyControlResponse::build(
                                source_tid,
                                0,
                                ProcessIdentifier::VFSD,
                                MessageType::Ipc,
                            )
                        },
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
