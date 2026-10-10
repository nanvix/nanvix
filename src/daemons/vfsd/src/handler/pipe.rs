// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Pipe operation handlers.
//!
//! These functions create pipes and service `read`/`write`/`close` on pipe descriptors, parking
//! callers that would block and reviving them from the complementary operation. They mirror the
//! MINIX VFS `suspend`/`revive` model on vfsd's single-threaded event loop: a parked reader stays
//! blocked inside its `__kcall_pull`, and a parked writer inside its `__kcall_recv`, until the
//! counterpart makes progress (or the pipe transitions to EOF/broken).

extern crate alloc;

use crate::{
    error::{
        build_error,
        fat32_to_error_code,
        ResponseContext,
    },
    pipe_wait::{
        BlockedReader,
        BlockedWriter,
        PipeWaitTable,
    },
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
        RequestIdentifier,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::syscall::{
    poll::input_message::PipeReadRetry,
    unistd::message::{
        PipeResponse,
        ReadResponse,
        WriteResponse,
    },
};
use ::vfs::{
    fd::{
        set_current_process,
        vfs_get_status_flags,
        vfs_pipe,
        vfs_pipe_consume,
        vfs_pipe_peek,
        vfs_pipe_write,
        PipeReadOutcome,
        PipeWriteOutcome,
    },
    Fat32Error,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum number of bytes transferred in a single pipe read/write request.
///
/// Matches the page-aligned chunk size the syscall layer uses, so a single request never exceeds
/// this. It must stay within `PIPE_BUF` so that each request is performed atomically.
const PIPE_BULK_SIZE: usize = PAGE_SIZE;

/// Revive pushes are non-blocking probes; callers without a registered pull stay queued for retry.
const PIPE_REVIVE_PUSH_TIMEOUT: Duration = Duration::ZERO;

/// Maximum time an initial pipe-write handler waits for the caller's tagged push.
const PIPE_REQUEST_PULL_TIMEOUT: Duration = Duration::from_millis(100);

/// Compile-time guarantee that a single pipe request is always atomic.
///
/// Keeping `PIPE_BULK_SIZE` within `PIPE_BUF` ensures a write never splits across a `PIPE_BUF`
/// boundary and so never interleaves with another writer's data (POSIX pipe-write atomicity), and
/// keeps the partial-write path in [`handle_pipe_write`] unreachable. If a platform's `PAGE_SIZE`
/// ever exceeds `PIPE_BUF`, raise `PIPE_BUF` rather than relaxing this bound.
const _: () = assert!(PIPE_BULK_SIZE <= ::vfs::pipe::PIPE_BUF);

/// Static buffer used for pipe data transfers.
///
/// Safety: vfsd is single-threaded (one message at a time), so there is never concurrent access.
/// Each access below is a momentary borrow that is released before any callee that also touches the
/// buffer runs.
static mut PIPE_BULK_BUFFER: [u8; PIPE_BULK_SIZE] = [0u8; PIPE_BULK_SIZE];

//==================================================================================================
// Helpers
//==================================================================================================

/// Returns `true` if the open file description for `fd` has `O_NONBLOCK` set.
fn is_nonblocking(fd: i32) -> bool {
    vfs_get_status_flags(fd) & ::sysapi::fcntl::file_status_flags::O_NONBLOCK != 0
}

/// Outcome of pushing data to a parked reader.
enum PushOutcome {
    /// The caller received the data.
    Delivered,
    /// The caller has not registered its pull yet and must remain queued.
    Retry,
    /// The caller cannot be reached and should be dropped.
    Failed,
}

/// Pushes an empty buffer to `(pid, tid)` to release a caller blocked in `__kcall_pull`.
fn push_empty(
    pid: ProcessIdentifier,
    tid: ThreadIdentifier,
    request_id: RequestIdentifier,
) -> PushOutcome {
    let result: Result<(), Error> = ::sys::kcall::ipc::__kcall_push_tagged_timed(
        pid,
        tid,
        &[],
        request_id,
        Some(PIPE_REVIVE_PUSH_TIMEOUT),
    );
    match result {
        Ok(()) => PushOutcome::Delivered,
        Err(error) if error.code == ErrorCode::OperationTimedOut => PushOutcome::Retry,
        Err(error) => {
            ::syslog::error!(
                "pipe: unblock push failed (pid={:?}, tid={:?}, error={:?})",
                pid,
                tid,
                error
            );
            PushOutcome::Failed
        },
    }
}

/// Builds a `ReadResponse` carrying `n` (the bytes already pushed to the caller).
fn read_response(tid: ThreadIdentifier, n: usize) -> Message {
    ReadResponse::build(
        tid,
        n as i32,
        [0u8; ReadResponse::BUFFER_SIZE],
        ProcessIdentifier::VFSD,
        MessageType::Ipc,
    )
}

/// Builds a `WriteResponse` carrying `n` (the bytes accepted into the pipe).
fn write_response(tid: ThreadIdentifier, n: usize) -> Message {
    WriteResponse::build(tid, n as i32, ProcessIdentifier::VFSD, MessageType::Ipc)
}

/// Outcome of attempting to serve a reader from the pipe buffer.
///
/// The data push to the caller (for [`Served`](ServeOutcome::Served), [`Eof`](ServeOutcome::Eof),
/// and [`Error`](ServeOutcome::Error)) is performed inside [`try_serve_reader`]; only
/// [`WouldBlock`](ServeOutcome::WouldBlock) and [`Retry`](ServeOutcome::Retry) leave the caller
/// blocked in `__kcall_pull`.
enum ServeOutcome {
    /// Pushed `N` bytes to the caller (`N > 0`).
    Served(usize),
    /// Pushed an empty buffer; the caller must receive end-of-file.
    Eof,
    /// No data available; the caller is still blocked in `__kcall_pull`.
    WouldBlock,
    /// The descriptor was not a readable pipe end; pushed an empty buffer.
    Error,
    /// The parked caller has not registered its pull yet and must remain queued.
    Retry,
    /// The parked caller cannot be reached and was dropped.
    Abandoned,
}

/// Attempts a non-blocking read of `fd` into the shared buffer and, on success, pushes the bytes to
/// the caller. The current process must already be set to the caller's process.
fn try_serve_reader(
    pid: ProcessIdentifier,
    tid: ThreadIdentifier,
    fd: i32,
    count: usize,
    request_id: RequestIdentifier,
    reviving: bool,
) -> ServeOutcome {
    let cap: usize = count.min(PIPE_BULK_SIZE);
    // SAFETY: single-threaded; the borrow is released when this function returns, and the only
    // callee touched while it is held (`__kcall_push`) does not access the buffer itself.
    let buf: &mut [u8] = unsafe { &mut PIPE_BULK_BUFFER[..cap] };
    // A revive must not consume the bytes before they are known to have reached the caller: the
    // push may time out on a caller that a signal pulled out of its `__kcall_pull`.
    let read: Result<PipeReadOutcome, Fat32Error> = vfs_pipe_peek(fd, buf);
    match read {
        Ok(PipeReadOutcome::Read(n)) => {
            let push: Result<(), Error> = ::sys::kcall::ipc::__kcall_push_tagged_timed(
                pid,
                tid,
                &buf[..n],
                request_id,
                Some(PIPE_REVIVE_PUSH_TIMEOUT),
            );
            match push {
                Ok(()) => {},
                Err(error) if error.code == ErrorCode::OperationTimedOut => {
                    return ServeOutcome::Retry;
                },
                Err(error) => {
                    ::syslog::error!(
                        "pipe read: push failed (pid={:?}, tid={:?}, fd={}, error={:?})",
                        pid,
                        tid,
                        fd,
                        error
                    );
                    return ServeOutcome::Abandoned;
                },
            }
            match vfs_pipe_consume(fd, n) {
                Ok(consumed) if consumed == n => {},
                Ok(consumed) => ::syslog::error!(
                    "pipe read: consume count mismatch (fd={}, expected={}, consumed={})",
                    fd,
                    n,
                    consumed
                ),
                Err(error) => ::syslog::error!(
                    "pipe read: consume failed (fd={}, count={}, error={:?})",
                    fd,
                    n,
                    error
                ),
            }
            ServeOutcome::Served(n)
        },
        Ok(PipeReadOutcome::Eof) => match push_empty(pid, tid, request_id) {
            PushOutcome::Delivered => ServeOutcome::Eof,
            PushOutcome::Retry => ServeOutcome::Retry,
            PushOutcome::Failed if reviving => ServeOutcome::Abandoned,
            PushOutcome::Failed => ServeOutcome::Eof,
        },
        Ok(PipeReadOutcome::WouldBlock) => ServeOutcome::WouldBlock,
        Err(_) => match push_empty(pid, tid, request_id) {
            PushOutcome::Delivered => ServeOutcome::Error,
            PushOutcome::Retry => ServeOutcome::Retry,
            PushOutcome::Failed if reviving => ServeOutcome::Abandoned,
            PushOutcome::Failed => ServeOutcome::Error,
        },
    }
}

//==================================================================================================
// Create
//==================================================================================================

/// Handles a `pipe()` request: allocates a pipe and its two descriptors in the caller's process.
///
/// The current process is bound by the dispatcher before this runs, so the descriptors land in the
/// caller's table.
pub(crate) fn handle_pipe_create(source: ThreadIdentifier) -> Message {
    match vfs_pipe() {
        Ok((read_fd, write_fd)) => {
            // Both descriptors are allocated by the same `vfs_pipe` table mutation, so they share
            // the post-mutation generation; report it so libposix stamps both cache entries.
            let epoch: u64 = ::vfs::fd::vfs_current_generation();
            ::syslog::trace!("pipe(): read_fd={}, write_fd={}", read_fd, write_fd);
            PipeResponse::build(
                source,
                read_fd,
                write_fd,
                epoch,
                ProcessIdentifier::VFSD,
                MessageType::Ipc,
            )
        },
        Err(e) => build_error(source, fat32_to_error_code(&e)),
    }
}

//==================================================================================================
// Read
//==================================================================================================

/// Handles a `read()` on a pipe read end.
///
/// Returns `Some(response)` when the request completes immediately (data available, EOF, `EAGAIN`,
/// or error) and `None` when the caller is parked (it stays blocked in `__kcall_pull` until a
/// writer or a close revives it).
pub(crate) fn handle_pipe_read(
    response_context: ResponseContext,
    fd: i32,
    count: usize,
    is_write: bool,
    pipe_id: u64,
    pipe_wait: &mut PipeWaitTable,
) -> Option<Message> {
    let source_pid: ProcessIdentifier = response_context.source_pid();
    let source_tid: ThreadIdentifier = response_context.source_tid();
    let request_id: RequestIdentifier = response_context.request_id();
    // Reading the write end is rejected with `EBADF`, regardless of `count`. The caller is blocked
    // in `__kcall_pull`, so release it before sending the error response.
    if is_write {
        return match push_empty(source_pid, source_tid, request_id) {
            PushOutcome::Delivered | PushOutcome::Failed => {
                Some(build_error(source_tid, ErrorCode::BadFile))
            },
            PushOutcome::Retry => {
                pipe_wait.park_reader(
                    pipe_id,
                    BlockedReader {
                        response_context,
                        source_tid,
                        source_pid,
                        fd,
                        count,
                        error: Some(ErrorCode::BadFile),
                    },
                );
                schedule_pipe_read_retry(pipe_id, pipe_wait);
                None
            },
        };
    }

    // A zero-length read returns immediately without blocking.
    if count == 0 {
        return match push_empty(source_pid, source_tid, request_id) {
            PushOutcome::Delivered | PushOutcome::Failed => Some(read_response(source_tid, 0)),
            PushOutcome::Retry => {
                pipe_wait.park_reader(
                    pipe_id,
                    BlockedReader {
                        response_context,
                        source_tid,
                        source_pid,
                        fd,
                        count,
                        error: None,
                    },
                );
                schedule_pipe_read_retry(pipe_id, pipe_wait);
                None
            },
        };
    }

    match try_serve_reader(source_pid, source_tid, fd, count, request_id, false) {
        ServeOutcome::Served(n) => {
            // Freed buffer space: a suspended writer may now make progress.
            rebalance(pipe_id, pipe_wait);
            Some(read_response(source_tid, n))
        },
        ServeOutcome::Eof => Some(read_response(source_tid, 0)),
        ServeOutcome::WouldBlock => {
            if is_nonblocking(fd) {
                // The caller is blocked in `__kcall_pull`; release it before the error response.
                match push_empty(source_pid, source_tid, request_id) {
                    PushOutcome::Delivered | PushOutcome::Failed => {
                        Some(build_error(source_tid, ErrorCode::TryAgain))
                    },
                    PushOutcome::Retry => {
                        pipe_wait.park_reader(
                            pipe_id,
                            BlockedReader {
                                response_context,
                                source_tid,
                                source_pid,
                                fd,
                                count,
                                error: Some(ErrorCode::TryAgain),
                            },
                        );
                        schedule_pipe_read_retry(pipe_id, pipe_wait);
                        None
                    },
                }
            } else {
                pipe_wait.park_reader(
                    pipe_id,
                    BlockedReader {
                        response_context,
                        source_tid,
                        source_pid,
                        fd,
                        count,
                        error: None,
                    },
                );
                None
            }
        },
        ServeOutcome::Error => Some(build_error(source_tid, ErrorCode::BadFile)),
        ServeOutcome::Retry => {
            pipe_wait.park_reader(
                pipe_id,
                BlockedReader {
                    response_context,
                    source_tid,
                    source_pid,
                    fd,
                    count,
                    error: None,
                },
            );
            schedule_pipe_read_retry(pipe_id, pipe_wait);
            None
        },
        ServeOutcome::Abandoned => Some(build_error(source_tid, ErrorCode::IoErr)),
    }
}

//==================================================================================================
// Write
//==================================================================================================

/// Handles a `write()` on a pipe write end.
///
/// Pulls the caller's bytes immediately (releasing its `__kcall_push`), buffers what it can, and
/// either answers now or parks the caller (which stays blocked in `__kcall_recv`) until a reader
/// drains space or all readers close.
pub(crate) fn handle_pipe_write(
    response_context: ResponseContext,
    fd: i32,
    count: usize,
    is_write: bool,
    pipe_id: u64,
    pipe_wait: &mut PipeWaitTable,
) -> Option<Message> {
    let source_pid: ProcessIdentifier = response_context.source_pid();
    let source_tid: ThreadIdentifier = response_context.source_tid();
    let cap: usize = count.min(PIPE_BULK_SIZE);

    // Pull the caller's data first; this releases the client's blocking `__kcall_push`.
    let pulled: usize = {
        // SAFETY: single-threaded; the borrow is released at the end of this block.
        let buf: &mut [u8] = unsafe { &mut PIPE_BULK_BUFFER[..cap] };
        match ::sys::kcall::ipc::__kcall_pull_tagged_timed(
            source_pid,
            source_tid,
            buf,
            response_context.request_id(),
            Some(PIPE_REQUEST_PULL_TIMEOUT),
        ) {
            Ok(p) => p.min(cap),
            Err(e) if e.code == ErrorCode::OperationTimedOut => return None,
            Err(e) => {
                ::syslog::error!("pipe write: pull failed (error={:?})", e);
                return Some(build_error(source_tid, ErrorCode::IoErr));
            },
        }
    };

    // Writing the read end is rejected with `EBADF`. The pull above already drained the client's
    // `__kcall_push`, so no data transfer is left dangling.
    if !is_write {
        return Some(build_error(source_tid, ErrorCode::BadFile));
    }

    // A zero-length write returns 0 without testing for a broken pipe (POSIX-permitted leniency).
    if pulled == 0 {
        return Some(write_response(source_tid, 0));
    }

    // SAFETY: single-threaded; this momentary borrow is not held across any buffer-touching call.
    let outcome: Result<PipeWriteOutcome, ::vfs::Fat32Error> =
        vfs_pipe_write(fd, unsafe { &PIPE_BULK_BUFFER[..pulled] });

    match outcome {
        Ok(PipeWriteOutcome::Wrote(n)) if n >= pulled => {
            // Everything fit: data is available, so revive any suspended readers.
            rebalance(pipe_id, pipe_wait);
            Some(write_response(source_tid, pulled))
        },
        Ok(PipeWriteOutcome::Wrote(n)) => {
            // Partial write (only reachable for requests larger than PIPE_BUF, which the atomicity
            // bound above rules out today). The `n` bytes that fit are already buffered, so revive
            // readers for them either way.
            if is_nonblocking(fd) {
                // POSIX: a non-blocking write transfers what fits and returns immediately.
                rebalance(pipe_id, pipe_wait);
                Some(write_response(source_tid, n))
            } else {
                // Park the remainder and let reviving readers drain space until it completes.
                // SAFETY: single-threaded; momentary borrow copied into an owned vector.
                let data: alloc::vec::Vec<u8> = unsafe { PIPE_BULK_BUFFER[..pulled].to_vec() };
                pipe_wait.park_writer(
                    pipe_id,
                    BlockedWriter {
                        response_context,
                        source_tid,
                        source_pid,
                        fd,
                        data,
                        written: n,
                        total: pulled,
                    },
                );
                rebalance(pipe_id, pipe_wait);
                None
            }
        },
        Ok(PipeWriteOutcome::WouldBlock) => {
            if is_nonblocking(fd) {
                Some(build_error(source_tid, ErrorCode::TryAgain))
            } else {
                // SAFETY: single-threaded; momentary borrow copied into an owned vector.
                let data: alloc::vec::Vec<u8> = unsafe { PIPE_BULK_BUFFER[..pulled].to_vec() };
                pipe_wait.park_writer(
                    pipe_id,
                    BlockedWriter {
                        response_context,
                        source_tid,
                        source_pid,
                        fd,
                        data,
                        written: 0,
                        total: pulled,
                    },
                );
                None
            }
        },
        Ok(PipeWriteOutcome::BrokenPipe) => Some(build_error(source_tid, ErrorCode::BrokenPipe)),
        Err(_) => Some(build_error(source_tid, ErrorCode::BadFile)),
    }
}

//==================================================================================================
// Revive / Wakeups
//==================================================================================================

/// Drains as many suspended readers as the buffer allows, pushing data and answering each.
///
/// Returns `true` if any reader was answered (used to drive [`rebalance`] to a fixpoint).
fn wake_readers(pipe_id: u64, pipe_wait: &mut PipeWaitTable) -> bool {
    let mut progress: bool = false;
    while let Some(reader) = pipe_wait.front_reader(pipe_id) {
        set_current_process(reader.source_pid);
        if let Some(error) = reader.error {
            match push_empty(
                reader.source_pid,
                reader.source_tid,
                reader.response_context.request_id(),
            ) {
                PushOutcome::Delivered => {
                    pipe_wait.pop_reader(pipe_id);
                    reader
                        .response_context
                        .send(&build_error(reader.source_tid, error));
                    progress = true;
                    continue;
                },
                PushOutcome::Retry => {
                    schedule_pipe_read_retry(pipe_id, pipe_wait);
                    break;
                },
                PushOutcome::Failed => {
                    pipe_wait.pop_reader(pipe_id);
                    progress = true;
                    continue;
                },
            }
        }
        match try_serve_reader(
            reader.source_pid,
            reader.source_tid,
            reader.fd,
            reader.count,
            reader.response_context.request_id(),
            true,
        ) {
            ServeOutcome::Served(n) => {
                pipe_wait.pop_reader(pipe_id);
                reader
                    .response_context
                    .send(&read_response(reader.source_tid, n));
                progress = true;
            },
            ServeOutcome::Eof => {
                pipe_wait.pop_reader(pipe_id);
                reader
                    .response_context
                    .send(&read_response(reader.source_tid, 0));
                progress = true;
            },
            ServeOutcome::WouldBlock => break,
            ServeOutcome::Error => {
                pipe_wait.pop_reader(pipe_id);
                reader
                    .response_context
                    .send(&build_error(reader.source_tid, ErrorCode::BadFile));
                progress = true;
            },
            ServeOutcome::Retry => {
                schedule_pipe_read_retry(pipe_id, pipe_wait);
                break;
            },
            ServeOutcome::Abandoned => {
                pipe_wait.pop_reader(pipe_id);
                progress = true;
            },
        }
    }
    progress
}

/// Yields to let a parked reader register its pull, then retries through VFSD's event loop.
fn schedule_pipe_read_retry(pipe_id: u64, pipe_wait: &mut PipeWaitTable) {
    if !pipe_wait.schedule_read_retry(pipe_id) {
        return;
    }
    if let Err(error) = ::sys::kcall::sched::__kcall_sched_yield() {
        ::syslog::warn!("pipe read: failed to yield before retry (error={:?})", error);
    }
    let tid: ThreadIdentifier = match ::sys::kcall::pm::__kcall_gettid() {
        Ok(tid) => tid,
        Err(error) => {
            pipe_wait.consume_read_retry(pipe_id);
            ::syslog::error!("pipe read: failed to get VFSD tid (error={:?})", error);
            return;
        },
    };
    let retry: Message = PipeReadRetry::build(tid, pipe_id);
    if let Err(error) = ::sys::kcall::ipc::__kcall_send(&retry) {
        pipe_wait.consume_read_retry(pipe_id);
        ::syslog::error!("pipe read: failed to schedule retry (error={:?})", error);
    }
}

/// Retries delivery to parked pipe readers after they had time to register their pulls.
pub(crate) fn retry_readers(pipe_id: u64, pipe_wait: &mut PipeWaitTable) {
    pipe_wait.consume_read_retry(pipe_id);
    rebalance(pipe_id, pipe_wait);
}

/// Advances as many suspended writers as the buffer allows, completing or re-parking each.
///
/// Returns `true` if any writer completed or made progress.
fn wake_writers(pipe_id: u64, pipe_wait: &mut PipeWaitTable) -> bool {
    let mut progress: bool = false;
    while let Some((pid, fd)) = pipe_wait.front_writer_meta(pipe_id) {
        set_current_process(pid);

        // Attempt to buffer the writer's remaining bytes.
        let outcome: Result<PipeWriteOutcome, ::vfs::Fat32Error> = {
            let w: &crate::pipe_wait::BlockedWriter = match pipe_wait.front_writer(pipe_id) {
                Some(w) => w,
                None => break,
            };
            vfs_pipe_write(fd, &w.data[w.written..])
        };

        match outcome {
            Ok(PipeWriteOutcome::Wrote(n)) => {
                let (done, response_context, tid, total): (
                    bool,
                    ResponseContext,
                    ThreadIdentifier,
                    usize,
                ) = {
                    let w: &mut crate::pipe_wait::BlockedWriter =
                        match pipe_wait.front_writer_mut(pipe_id) {
                            Some(w) => w,
                            None => break,
                        };
                    w.written += n;
                    (w.written >= w.data.len(), w.response_context, w.source_tid, w.total)
                };
                if done {
                    pipe_wait.pop_writer(pipe_id);
                    response_context.send(&write_response(tid, total));
                    progress = true;
                } else {
                    progress |= n > 0;
                    // Buffer is full again; the writer remains parked with its remainder.
                    break;
                }
            },
            Ok(PipeWriteOutcome::WouldBlock) => break,
            Ok(PipeWriteOutcome::BrokenPipe) => {
                let (response_context, tid): (ResponseContext, ThreadIdentifier) =
                    match pipe_wait.front_writer(pipe_id) {
                        Some(w) => (w.response_context, w.source_tid),
                        None => break,
                    };
                pipe_wait.pop_writer(pipe_id);
                response_context.send(&build_error(tid, ErrorCode::BrokenPipe));
                progress = true;
            },
            Err(_) => {
                let (response_context, tid): (ResponseContext, ThreadIdentifier) =
                    match pipe_wait.front_writer(pipe_id) {
                        Some(w) => (w.response_context, w.source_tid),
                        None => break,
                    };
                pipe_wait.pop_writer(pipe_id);
                response_context.send(&build_error(tid, ErrorCode::BadFile));
                progress = true;
            },
        }
    }
    progress
}

/// Drives the complementary wakeups to a fixpoint after a pipe buffer mutation.
///
/// Reviving writers adds data (which may free a reader to run) and reviving readers frees space
/// (which may let a writer run); the loop terminates once neither side can make further progress.
fn rebalance(pipe_id: u64, pipe_wait: &mut PipeWaitTable) {
    loop {
        let writers_progressed: bool = wake_writers(pipe_id, pipe_wait);
        let readers_progressed: bool = wake_readers(pipe_id, pipe_wait);
        if !writers_progressed && !readers_progressed {
            break;
        }
    }
}

/// Wakes all readers suspended on `pipe_id` after the last writer closes.
///
/// Buffered data is delivered first; remaining readers are answered with a zero-length read.
pub(crate) fn wake_all_readers_eof(pipe_id: u64, pipe_wait: &mut PipeWaitTable) {
    let _ = wake_readers(pipe_id, pipe_wait);
}

/// Fails all writers suspended on `pipe_id` with `EPIPE`.
///
/// Invoked when the last read end closes (or its owner exits): every parked writer is answered with
/// a broken-pipe error.
pub(crate) fn fail_all_writers_epipe(pipe_id: u64, pipe_wait: &mut PipeWaitTable) {
    for writer in pipe_wait.drain_writers(pipe_id) {
        writer
            .response_context
            .send(&build_error(writer.source_tid, ErrorCode::BrokenPipe));
    }
}
