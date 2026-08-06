// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! HostFs-aware handler wrappers.
//!
//! These functions check whether a file descriptor is backed by the host filesystem
//! and forward operations via IKC to hostfsd. If the FD is not hostfs-backed, they
//! delegate to the standard FAT32 handlers.
//!
//! # Inline Data Limits
//!
//! Read operations are limited to [`MAX_INLINE_READ_DATA`](::hostfs_api::MAX_INLINE_READ_DATA)
//! bytes per request, and write operations to
//! [`MAX_INLINE_WRITE_DATA`](::hostfs_api::MAX_INLINE_WRITE_DATA) bytes. Larger requests are
//! silently clamped. Callers (the guest VFS layer) must handle short reads/writes and issue
//! additional requests for the remainder.
//!
//! When forwarding to hostfsd, these handlers send the IKC request and push a
//! [`PendingOp`] onto the pending queue. They return `None` to indicate that no
//! immediate response should be sent — the main event loop will complete the
//! operation when the IKC response arrives.

extern crate alloc;

use crate::{
    console_wait::ConsoleWaitTable,
    error::{
        build_error,
        fat32_to_error_code,
        ResponseContext,
    },
    hostfs,
    pending::{
        PendingOp,
        PendingOpKind,
        PendingQueue,
    },
    pipe_wait::PipeWaitTable,
};
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
use ::sysapi::fcntl::atflags::AT_FDCWD;
use ::syscall::{
    unistd::message::{
        ChangeDirectoryRequest,
        CloseRequest,
        Dup2Request,
        Dup2Response,
        FileSyncRequest,
        FileTruncateRequest,
        ReadRequest,
        SeekRequest,
        WriteRequest,
    },
    SystemCallMessage,
};

//==================================================================================================
// HostFs-Aware Short Request Handlers
//==================================================================================================

pub(crate) fn handle_close_with_hostfs(
    response_context: ResponseContext,
    msg: SystemCallMessage,
    pending: &mut PendingQueue,
    pipe_wait: &mut PipeWaitTable,
) -> Option<Message> {
    let source_pid: ProcessIdentifier = response_context.source_pid();
    let source: ThreadIdentifier = response_context.source_tid();
    let req: CloseRequest = CloseRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;

    // Pipe close: if this drops the last reference to an end, fire the matching wakeup so any
    // suspended counterparts observe EOF (write end gone) or `EPIPE` (read end gone).
    if let Some((pipe_id, is_write)) = ::vfs::fd::vfs_pipe_id(fd) {
        let last_ref: bool = ::vfs::fd::vfs_pipe_is_last_ref(fd);
        let response: Message = super::short::handle_close(source, msg);
        if last_ref {
            if is_write {
                super::pipe::wake_all_readers_eof(pipe_id, pipe_wait);
            } else {
                super::pipe::fail_all_writers_epipe(pipe_id, pipe_wait);
            }
        }
        return Some(response);
    }

    if let Some(remote_fd) = ::vfs::fd::vfs_hostfs_remote_fd(fd) {
        // A forked child may share this open file description with its parent. Only forward the
        // close to hostfsd when this is the last descriptor referencing it; otherwise just drop the
        // local descriptor and acknowledge.
        if !::vfs::fd::vfs_hostfs_is_last_ref(fd) {
            return Some(super::short::handle_close(source, msg));
        }
        if !pending.has_capacity() {
            return Some(build_error(source, ErrorCode::ResourceBusy));
        }
        let op_id: ::hostfs_api::OperationId = pending.alloc_op_id();
        // Reserve the pending slot BEFORE sending the IKC request so that insert
        // cannot fail after send, avoiding orphaned IKC requests with no completion record.
        if pending
            .insert(
                op_id,
                PendingOp {
                    response_context,
                    source_tid: source,
                    source_pid,
                    kind: PendingOpKind::Close,
                },
            )
            .is_err()
        {
            return Some(build_error(source, ErrorCode::ResourceBusy));
        }
        // Send the IKC close request. If the send fails, remove the pending slot
        // and return an error so the caller can retry.
        if hostfs::send_close_request(remote_fd, op_id).is_err() {
            ::syslog::error!("hostfs close: IKC send failed (fd={}, remote_fd={})", fd, remote_fd);
            pending.remove(op_id);
            return Some(build_error(source, ErrorCode::IoErr));
        }
        // Release the local FD only after both the pending insert and IKC send succeed.
        if let Err(e) = ::vfs::fd::vfs_close(fd) {
            ::syslog::warn!("hostfs close: local vfs_close failed (fd={}, error={:?})", fd, e);
        }
        return None;
    }

    // Socket close: the slot is a flat descriptor vfsd owns, but the endpoint lives in networkd.
    // Capture whether this drops the last reference before releasing the slot; if so, forward the
    // endpoint close to networkd so the remote descriptor does not leak. The networkd close is
    // fire-and-forget, so the response is returned synchronously.
    if let Some(remote_fd) = ::vfs::fd::vfs_socket_remote_fd(fd) {
        let last_ref: bool = ::vfs::fd::vfs_socket_is_last_ref(fd);
        let response: Message = super::short::handle_close(source, msg);
        if last_ref {
            if let Err(e) = crate::networkd::send_close_request(remote_fd) {
                ::syslog::warn!(
                    "socket close: failed to forward endpoint close to networkd (remote_fd={}, \
                     error={:?})",
                    remote_fd,
                    e
                );
            }
        }
        return Some(response);
    }

    Some(super::short::handle_close(source, msg))
}

/// Handles `dup2(oldfd, newfd)` as an authoritative slot-table operation.
///
/// `newfd` is re-pointed at `oldfd`'s open file description, which works uniformly across every
/// backend — including the cross-backend redirections the old split descriptor model could not
/// express (e.g. `dup2(file_fd, 1)` so a subsequent `write(1)` lands in the file). The descriptor
/// previously held by `newfd` is closed with the *same* last-reference accounting as a real
/// `close`: a displaced pipe end fires its EOF/`EPIPE` wakeup, and a displaced host-backed last
/// reference has its remote handle closed on hostfsd. That remote close is fire-and-forget — POSIX
/// specifies that `dup2`'s implicit close ignores errors — so no pending op is registered and the
/// main loop discards hostfsd's acknowledgement via the `FIRE_AND_FORGET` sentinel.
///
/// Because the remote reclaim never blocks, this returns a response synchronously rather than
/// deferring like the hostfs-aware close path.
pub(crate) fn handle_dup2(
    source: ThreadIdentifier,
    msg: SystemCallMessage,
    pipe_wait: &mut PipeWaitTable,
) -> Message {
    let req: Dup2Request = Dup2Request::from_bytes(msg.payload);
    let oldfd: i32 = req.oldfd;
    let newfd: i32 = req.newfd;

    // POSIX: an invalid source descriptor fails with `EBADF` and leaves `newfd` untouched.
    if ::vfs::fd::vfs_resolve(oldfd).is_none() {
        return build_error(source, ErrorCode::BadFile);
    }

    // `dup2(fd, fd)` returns `fd` and performs no implicit close.
    if oldfd == newfd {
        return Dup2Response::build(source, newfd, ProcessIdentifier::VFSD, MessageType::Ipc);
    }

    // Capture whatever last-reference reclaim the descriptor displaced from `newfd` owes — exactly
    // as a close of `newfd` would — before re-pointing the slot. If `newfd` already aliases
    // `oldfd`'s description it is not a last reference, so nothing is reclaimed.
    let displaced_pipe: Option<(u64, bool)> =
        ::vfs::fd::vfs_pipe_id(newfd).filter(|_| ::vfs::fd::vfs_pipe_is_last_ref(newfd));
    let displaced_hostfs: Option<i32> =
        ::vfs::fd::vfs_hostfs_remote_fd(newfd).filter(|_| ::vfs::fd::vfs_hostfs_is_last_ref(newfd));
    let displaced_socket: Option<i32> =
        ::vfs::fd::vfs_socket_remote_fd(newfd).filter(|_| ::vfs::fd::vfs_socket_is_last_ref(newfd));

    // Perform the authoritative table mutation: `newfd` now aliases `oldfd`'s description. This
    // drops the displaced description locally; its external reclaim is performed below.
    if let Err(e) = ::vfs::fd::vfs_dup2(oldfd, newfd) {
        return build_error(source, fat32_to_error_code(&e));
    }

    // A displaced pipe end fires the same EOF/`EPIPE` wakeup that closing it would.
    if let Some((pipe_id, was_write)) = displaced_pipe {
        if was_write {
            super::pipe::wake_all_readers_eof(pipe_id, pipe_wait);
        } else {
            super::pipe::fail_all_writers_epipe(pipe_id, pipe_wait);
        }
    }

    // A displaced host-backed last reference must have its remote handle closed so it does not leak.
    if let Some(remote_fd) = displaced_hostfs {
        if let Err(e) =
            hostfs::send_close_request(remote_fd, ::hostfs_api::OperationId::FIRE_AND_FORGET)
        {
            ::syslog::warn!(
                "dup2: failed to close displaced hostfs handle (remote_fd={}, error={:?})",
                remote_fd,
                e
            );
        }
    }

    // A displaced socket last reference must have its networkd endpoint closed so it does not leak.
    if let Some(remote_fd) = displaced_socket {
        if let Err(e) = crate::networkd::send_close_request(remote_fd) {
            ::syslog::warn!(
                "dup2: failed to close displaced socket endpoint (remote_fd={}, error={:?})",
                remote_fd,
                e
            );
        }
    }

    Dup2Response::build(source, newfd, ProcessIdentifier::VFSD, MessageType::Ipc)
}

pub(crate) fn handle_seek_with_hostfs(
    response_context: ResponseContext,
    msg: SystemCallMessage,
    pending: &mut PendingQueue,
) -> Option<Message> {
    let source_pid: ProcessIdentifier = response_context.source_pid();
    let source: ThreadIdentifier = response_context.source_tid();
    let req: SeekRequest = SeekRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;

    // A pipe is not seekable: report `ESPIPE` without touching the buffer.
    if ::vfs::fd::vfs_pipe_id(fd).is_some() {
        return Some(build_error(source, ErrorCode::IllegalSeek));
    }

    if let Some(remote_fd) = ::vfs::fd::vfs_hostfs_remote_fd(fd) {
        if !pending.has_capacity() {
            return Some(build_error(source, ErrorCode::ResourceBusy));
        }
        let op_id: ::hostfs_api::OperationId = pending.alloc_op_id();
        if hostfs::send_lseek_request(remote_fd, req.offset, req.whence, op_id).is_err() {
            return Some(build_error(source, ErrorCode::IoErr));
        }
        if pending
            .insert(
                op_id,
                PendingOp {
                    response_context,
                    source_tid: source,
                    source_pid,
                    kind: PendingOpKind::Seek,
                },
            )
            .is_err()
        {
            return Some(build_error(source, ErrorCode::ResourceBusy));
        }
        return None;
    }

    Some(super::short::handle_seek(source, msg))
}

pub(crate) fn handle_fsync_with_hostfs(
    response_context: ResponseContext,
    msg: SystemCallMessage,
    pending: &mut PendingQueue,
) -> Option<Message> {
    let source_pid: ProcessIdentifier = response_context.source_pid();
    let source: ThreadIdentifier = response_context.source_tid();
    let req: FileSyncRequest = FileSyncRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;

    if let Some(remote_fd) = ::vfs::fd::vfs_hostfs_remote_fd(fd) {
        if !pending.has_capacity() {
            return Some(build_error(source, ErrorCode::ResourceBusy));
        }
        let op_id: ::hostfs_api::OperationId = pending.alloc_op_id();
        if hostfs::send_flush_request(remote_fd, op_id).is_err() {
            return Some(build_error(source, ErrorCode::IoErr));
        }
        if pending
            .insert(
                op_id,
                PendingOp {
                    response_context,
                    source_tid: source,
                    source_pid,
                    kind: PendingOpKind::Flush,
                },
            )
            .is_err()
        {
            return Some(build_error(source, ErrorCode::ResourceBusy));
        }
        return None;
    }

    Some(super::short::handle_fsync(source, msg))
}

pub(crate) fn handle_ftruncate_with_hostfs(
    response_context: ResponseContext,
    msg: SystemCallMessage,
    pending: &mut PendingQueue,
) -> Option<Message> {
    let source_pid: ProcessIdentifier = response_context.source_pid();
    let source: ThreadIdentifier = response_context.source_tid();
    let req: FileTruncateRequest = FileTruncateRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;

    if let Some(remote_fd) = ::vfs::fd::vfs_hostfs_remote_fd(fd) {
        if req.length < 0 {
            return Some(build_error(source, ErrorCode::InvalidArgument));
        }
        if !pending.has_capacity() {
            return Some(build_error(source, ErrorCode::ResourceBusy));
        }
        let op_id: ::hostfs_api::OperationId = pending.alloc_op_id();
        if hostfs::send_truncate_request(remote_fd, req.length, op_id).is_err() {
            return Some(build_error(source, ErrorCode::IoErr));
        }
        if pending
            .insert(
                op_id,
                PendingOp {
                    response_context,
                    source_tid: source,
                    source_pid,
                    kind: PendingOpKind::Truncate,
                },
            )
            .is_err()
        {
            return Some(build_error(source, ErrorCode::ResourceBusy));
        }
        return None;
    }

    Some(super::short::handle_ftruncate(source, msg))
}

//==================================================================================================
// HostFs-Aware Read/Write Handlers
//==================================================================================================

pub(crate) fn handle_fstat_with_hostfs(
    response_context: ResponseContext,
    msg: SystemCallMessage,
    pending: &mut PendingQueue,
) -> Option<Vec<Message>> {
    let source_pid: ProcessIdentifier = response_context.source_pid();
    let source: ThreadIdentifier = response_context.source_tid();
    let req: FileStatRequest = FileStatRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;

    if let Some(remote_fd) = ::vfs::fd::vfs_hostfs_remote_fd(fd) {
        if !pending.has_capacity() {
            return Some(vec![build_error(source, ErrorCode::ResourceBusy)]);
        }
        let op_id: ::hostfs_api::OperationId = pending.alloc_op_id();
        if hostfs::send_stat_request(remote_fd, op_id).is_err() {
            return Some(vec![build_error(source, ErrorCode::IoErr)]);
        }
        if pending
            .insert(
                op_id,
                PendingOp {
                    response_context,
                    source_tid: source,
                    source_pid,
                    kind: PendingOpKind::Stat,
                },
            )
            .is_err()
        {
            return Some(vec![build_error(source, ErrorCode::ResourceBusy)]);
        }
        return None;
    }

    Some(super::long::handle_fstat(source, msg))
}

pub(crate) fn handle_fstatat_with_hostfs(
    response_context: ResponseContext,
    request: FileStatAtRequest,
    pending: &mut PendingQueue,
) -> Option<Vec<Message>> {
    let source_pid: ProcessIdentifier = response_context.source_pid();
    let source: ThreadIdentifier = response_context.source_tid();
    let Some(resolved) = vfs_resolve_path(request.dirfd, &request.path) else {
        return Some(vec![build_error(source, ErrorCode::InvalidArgument)]);
    };
    let final_path: &str = &resolved;

    if hostfs::is_hostfs_path(final_path) {
        // Both stat modes are supported over hostfs. No-follow (`AT_SYMLINK_NOFOLLOW`)
        // maps to a path-based lstat; following stat (the default for `stat(2)`) maps to
        // a path-based following stat. Both reuse the same response wire format
        // (`LstatResponse`) and completion path (`complete_lstat`); only the host-side
        // resolution differs (no-follow vs follow of the final component).
        let no_follow: bool = request.flag & ::sysapi::fcntl::atflags::AT_SYMLINK_NOFOLLOW != 0;
        if !pending.has_capacity() {
            return Some(vec![build_error(source, ErrorCode::ResourceBusy)]);
        }
        let op_id: ::hostfs_api::OperationId = pending.alloc_op_id();
        let (send_result, kind) = if no_follow {
            (hostfs::send_lstat_request(final_path, op_id), PendingOpKind::Lstat)
        } else {
            (hostfs::send_pathstat_request(final_path, op_id), PendingOpKind::PathStat)
        };
        match send_result {
            Ok(()) => {
                if pending
                    .insert(
                        op_id,
                        PendingOp {
                            response_context,
                            source_tid: source,
                            source_pid,
                            kind,
                        },
                    )
                    .is_err()
                {
                    return Some(vec![build_error(source, ErrorCode::ResourceBusy)]);
                }
                return None;
            },
            Err(e) => return Some(vec![build_error(source, e)]),
        }
    }

    Some(super::long::handle_fstatat(source, request))
}

/// hostfs-aware `chdir`.
///
/// For a target under the hostfs mount, forwards a path-based stat to hostfsd and
/// defers (returns `None`); the completion (`complete_chdir`) sets the cwd when the
/// target is a directory and returns `ENOTDIR` otherwise. Non-hostfs targets fall
/// through to the local VFS handler, which validates against the FAT mount table.
pub(crate) fn handle_chdir_with_hostfs(
    response_context: ResponseContext,
    request: ChangeDirectoryRequest,
    pending: &mut PendingQueue,
) -> Option<Vec<Message>> {
    let source_pid: ProcessIdentifier = response_context.source_pid();
    let source: ThreadIdentifier = response_context.source_tid();
    let Some(resolved) = vfs_resolve_path(AT_FDCWD, &request.path) else {
        return Some(vec![build_error(source, ErrorCode::InvalidArgument)]);
    };

    if hostfs::is_hostfs_path(&resolved) {
        if !pending.has_capacity() {
            return Some(vec![build_error(source, ErrorCode::ResourceBusy)]);
        }
        let op_id = pending.alloc_op_id();
        match hostfs::send_pathstat_request(&resolved, op_id) {
            Ok(()) => {
                if pending
                    .insert(
                        op_id,
                        PendingOp {
                            response_context,
                            source_tid: source,
                            source_pid,
                            kind: PendingOpKind::Chdir { path: resolved },
                        },
                    )
                    .is_err()
                {
                    return Some(vec![build_error(source, ErrorCode::ResourceBusy)]);
                }
                return None;
            },
            Err(e) => return Some(vec![build_error(source, e)]),
        }
    }

    Some(handle_chdir(source, request))
}

pub(crate) fn handle_read_with_hostfs(
    response_context: ResponseContext,
    msg: SystemCallMessage,
    pending: &mut PendingQueue,
    console_wait: &mut ConsoleWaitTable,
    pipe_wait: &mut PipeWaitTable,
) -> Option<Message> {
    let source_pid: ProcessIdentifier = response_context.source_pid();
    let source_tid: ThreadIdentifier = response_context.source_tid();
    let req: ReadRequest = ReadRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;

    if let Ok(stream) = ::vfs::fd::vfs_console_stream(fd) {
        return super::readwrite::handle_console_read(
            response_context,
            fd,
            stream,
            req.count as usize,
            console_wait,
        );
    }

    // Pipe read end: served by the pipe handler (which may park the caller).
    if let Some((pipe_id, is_write)) = ::vfs::fd::vfs_pipe_id(fd) {
        return super::pipe::handle_pipe_read(
            response_context,
            fd,
            req.count as usize,
            is_write,
            pipe_id,
            pipe_wait,
        );
    }

    if let Some(remote_fd) = ::vfs::fd::vfs_hostfs_remote_fd(fd) {
        if !pending.has_capacity() {
            let _ = ::sys::kcall::ipc::__kcall_push(source_pid, source_tid, &[]);
            return Some(build_error(source_tid, ErrorCode::ResourceBusy));
        }
        let op_id: ::hostfs_api::OperationId = pending.alloc_op_id();
        let count: usize = req.count as usize;
        let buf_size: usize = count.min(::hostfs_api::MAX_INLINE_READ_DATA);
        if hostfs::send_read_request(remote_fd, buf_size, op_id).is_err() {
            let _ = ::sys::kcall::ipc::__kcall_push(source_pid, source_tid, &[]);
            return Some(build_error(source_tid, ErrorCode::IoErr));
        }
        if pending
            .insert(
                op_id,
                PendingOp {
                    response_context,
                    source_tid,
                    source_pid,
                    kind: PendingOpKind::Read { count: buf_size },
                },
            )
            .is_err()
        {
            let _ = ::sys::kcall::ipc::__kcall_push(source_pid, source_tid, &[]);
            return Some(build_error(source_tid, ErrorCode::ResourceBusy));
        }
        return None;
    }

    Some(super::readwrite::handle_read(source_pid, source_tid, msg))
}

pub(crate) fn handle_write_with_hostfs(
    response_context: ResponseContext,
    msg: SystemCallMessage,
    pending: &mut PendingQueue,
    pipe_wait: &mut PipeWaitTable,
) -> Option<Message> {
    let source_pid: ProcessIdentifier = response_context.source_pid();
    let source_tid: ThreadIdentifier = response_context.source_tid();
    let req: WriteRequest = WriteRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;

    // Pipe write end: served by the pipe handler (which may park the caller).
    if let Some((pipe_id, is_write)) = ::vfs::fd::vfs_pipe_id(fd) {
        return super::pipe::handle_pipe_write(
            response_context,
            fd,
            req.count as usize,
            is_write,
            pipe_id,
            pipe_wait,
        );
    }

    if let Some(remote_fd) = ::vfs::fd::vfs_hostfs_remote_fd(fd) {
        if !pending.has_capacity() {
            return Some(build_error(source_tid, ErrorCode::ResourceBusy));
        }
        let op_id: ::hostfs_api::OperationId = pending.alloc_op_id();
        let count: usize = req.count as usize;
        let buf_size: usize = count.min(::hostfs_api::MAX_INLINE_WRITE_DATA);
        let mut buf: [u8; ::hostfs_api::MAX_INLINE_WRITE_DATA] =
            [0u8; ::hostfs_api::MAX_INLINE_WRITE_DATA];

        // Pull the data from the caller BEFORE sending the IKC request.
        match ::sys::kcall::ipc::__kcall_pull(source_pid, source_tid, &mut buf[..buf_size]) {
            Ok(pulled) => {
                let write_len: usize = pulled.min(buf_size);
                if hostfs::send_write_request(remote_fd, &buf[..write_len], op_id).is_err() {
                    return Some(build_error(source_tid, ErrorCode::IoErr));
                }
                if pending
                    .insert(
                        op_id,
                        PendingOp {
                            response_context,
                            source_tid,
                            source_pid,
                            kind: PendingOpKind::Write,
                        },
                    )
                    .is_err()
                {
                    return Some(build_error(source_tid, ErrorCode::ResourceBusy));
                }
                return None;
            },
            Err(e) => {
                ::syslog::error!("hostfs write: pull failed (error={:?})", e);
                return Some(build_error(source_tid, ErrorCode::IoErr));
            },
        }
    }

    Some(super::readwrite::handle_write(source_pid, source_tid, msg))
}

//==================================================================================================
// HostFs-Aware Long Request Handlers (path-based operations)
//==================================================================================================

use ::syscall::{
    dirent::message::GetDirectoryEntriesRequest,
    fcntl::message::{
        OpenAtRequest,
        RenameAtRequest,
        UnlinkAtRequest,
    },
    sys::stat::message::{
        FileStatAtRequest,
        FileStatRequest,
        MakeDirectoryAtRequest,
    },
    unistd::message::{
        ReadLinkAtRequest,
        SymbolicLinkAtRequest,
    },
};
use ::vfs::fd::vfs_resolve_path;
use alloc::{
    vec,
    vec::Vec,
};

use super::long::handle_chdir;

/// Handles getdents with hostfs awareness.
///
/// If the FD is backed by hostfs, the directory listing is served by an async sweep:
/// hostfsd returns one entry per IKC round-trip, so a single `getdents` call issues
/// repeated readdir requests (under one op_id) until the requested entry count is
/// reached or the directory is exhausted. The per-FD iteration cursor is persisted in
/// the VFS handle so successive `getdents` calls resume where the previous one stopped.
///
/// # Concurrency
///
/// Like POSIX `readdir`, concurrent directory reads on a *shared* FD are unspecified:
/// the per-FD cursor advances only when a sweep completes, so two overlapping
/// `getdents` calls on the same FD may observe duplicated or skipped entries. Programs
/// must not share a directory FD across threads doing simultaneous reads.
///
/// Returns `None` when the request was forwarded to hostfsd (the response is sent later
/// from the event loop). Non-hostfs FDs fall through to the synchronous VFS handler.
pub(crate) fn handle_getdents_with_hostfs(
    response_context: ResponseContext,
    msg: SystemCallMessage,
    pending: &mut PendingQueue,
) -> Option<Vec<Message>> {
    let source_pid: ProcessIdentifier = response_context.source_pid();
    let source: ThreadIdentifier = response_context.source_tid();
    let req: GetDirectoryEntriesRequest = GetDirectoryEntriesRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;

    if let Some(remote_fd) = ::vfs::fd::vfs_hostfs_remote_fd(fd) {
        // Reject getdents on hostfs non-directory FDs (e.g. regular files). The cursor
        // accessor returns `Some(_)` only for hostfs directory handles, so a `None`
        // here means the FD is a hostfs file. Fail rather than forward to hostfsd (which
        // would look like an empty directory). `InvalidDirectory` (ENOTDIR) is the
        // POSIX-correct error for `getdents` on a non-directory; note this differs from
        // the non-hostfs FAT path, which surfaces `InvalidArgument` for the same case.
        let Some(start_offset) = ::vfs::fd::vfs_hostfs_readdir_offset(fd) else {
            return Some(vec![build_error(source, ErrorCode::InvalidDirectory)]);
        };
        let count: usize = req.count as usize;
        if count == 0 {
            return Some(vec![build_error(source, ErrorCode::InvalidArgument)]);
        }
        // Do not trust the guest-supplied count: cap it to the protocol maximum so a
        // malformed request cannot drive an unbounded sweep.
        if count > GetDirectoryEntriesRequest::MAX_ENTRIES {
            return Some(vec![build_error(source, ErrorCode::TooBig)]);
        }
        if !pending.has_capacity() {
            return Some(vec![build_error(source, ErrorCode::ResourceBusy)]);
        }
        let op_id: ::hostfs_api::OperationId = pending.alloc_op_id();
        if hostfs::send_readdir_request(remote_fd, start_offset, op_id).is_err() {
            return Some(vec![build_error(source, ErrorCode::IoErr)]);
        }
        if pending
            .insert(
                op_id,
                PendingOp {
                    response_context,
                    source_tid: source,
                    source_pid,
                    kind: PendingOpKind::Getdents {
                        remote_fd,
                        guest_fd: fd,
                        next_offset: start_offset,
                        target_count: count,
                        entries: alloc::vec::Vec::new(),
                    },
                },
            )
            .is_err()
        {
            return Some(vec![build_error(source, ErrorCode::ResourceBusy)]);
        }
        return None;
    }

    Some(super::long::handle_getdents(source, msg))
}

pub(crate) fn handle_openat_with_hostfs(
    response_context: ResponseContext,
    mut request: OpenAtRequest,
    pending: &mut PendingQueue,
) -> Option<Vec<Message>> {
    let source_pid: ProcessIdentifier = response_context.source_pid();
    let source: ThreadIdentifier = response_context.source_tid();
    request.mode = ::vfs::fd::vfs_apply_umask(request.mode);
    let Some(resolved) = vfs_resolve_path(request.dirfd, &request.pathname) else {
        return Some(vec![build_error(source, ErrorCode::InvalidArgument)]);
    };
    let final_path: &str = &resolved;

    if hostfs::is_hostfs_path(final_path) {
        if !pending.has_capacity() {
            return Some(vec![build_error(source, ErrorCode::ResourceBusy)]);
        }
        let op_id: ::hostfs_api::OperationId = pending.alloc_op_id();
        let open_path: alloc::string::String = alloc::string::String::from(final_path);
        match hostfs::send_open_request(final_path, request.flags, request.mode, op_id) {
            Ok(()) => {
                if pending
                    .insert(
                        op_id,
                        PendingOp {
                            response_context,
                            source_tid: source,
                            source_pid,
                            kind: PendingOpKind::Open { path: open_path },
                        },
                    )
                    .is_err()
                {
                    return Some(vec![build_error(source, ErrorCode::ResourceBusy)]);
                }
                return None;
            },
            Err(e) => return Some(vec![build_error(source, e)]),
        }
    }

    Some(super::long::handle_openat(source, request))
}

pub(crate) fn handle_renameat_with_hostfs(
    response_context: ResponseContext,
    request: RenameAtRequest,
    pending: &mut PendingQueue,
) -> Option<Vec<Message>> {
    let source_pid: ProcessIdentifier = response_context.source_pid();
    let source: ThreadIdentifier = response_context.source_tid();
    let Some(old_resolved) = vfs_resolve_path(request.olddirfd, &request.oldpath) else {
        return Some(vec![build_error(source, ErrorCode::InvalidArgument)]);
    };
    let Some(new_resolved) = vfs_resolve_path(request.newdirfd, &request.newpath) else {
        return Some(vec![build_error(source, ErrorCode::InvalidArgument)]);
    };
    let old_final: &str = &old_resolved;
    let new_final: &str = &new_resolved;

    let old_is_hostfs: bool = hostfs::is_hostfs_path(old_final);
    let new_is_hostfs: bool = hostfs::is_hostfs_path(new_final);

    // Reject cross-filesystem renames (one path on hostfs, the other on ramfs).
    if old_is_hostfs != new_is_hostfs {
        return Some(vec![build_error(source, ErrorCode::OperationNotSupported)]);
    }

    if old_is_hostfs {
        if !pending.has_capacity() {
            return Some(vec![build_error(source, ErrorCode::ResourceBusy)]);
        }
        let op_id: ::hostfs_api::OperationId = pending.alloc_op_id();
        match hostfs::send_rename_request(old_final, new_final, op_id) {
            Ok(()) => {
                if pending
                    .insert(
                        op_id,
                        PendingOp {
                            response_context,
                            source_tid: source,
                            source_pid,
                            kind: PendingOpKind::Rename,
                        },
                    )
                    .is_err()
                {
                    return Some(vec![build_error(source, ErrorCode::ResourceBusy)]);
                }
                return None;
            },
            Err(e) => return Some(vec![build_error(source, e)]),
        }
    }

    Some(super::long::handle_renameat(source, request))
}

pub(crate) fn handle_unlinkat_with_hostfs(
    response_context: ResponseContext,
    request: UnlinkAtRequest,
    pending: &mut PendingQueue,
) -> Option<Vec<Message>> {
    let source_pid: ProcessIdentifier = response_context.source_pid();
    let source: ThreadIdentifier = response_context.source_tid();
    let Some(resolved) = vfs_resolve_path(request.dirfd, &request.pathname) else {
        return Some(vec![build_error(source, ErrorCode::InvalidArgument)]);
    };
    let final_path: &str = &resolved;

    if hostfs::is_hostfs_path(final_path) {
        if !pending.has_capacity() {
            return Some(vec![build_error(source, ErrorCode::ResourceBusy)]);
        }
        let op_id: ::hostfs_api::OperationId = pending.alloc_op_id();
        let is_rmdir: bool = (request.flags & ::sysapi::fcntl::atflags::AT_REMOVEDIR) != 0;
        let result = if is_rmdir {
            hostfs::send_rmdir_request(final_path, op_id)
        } else {
            hostfs::send_unlink_request(final_path, op_id)
        };
        match result {
            Ok(()) => {
                let kind = if is_rmdir {
                    PendingOpKind::Rmdir
                } else {
                    PendingOpKind::Unlink
                };
                if pending
                    .insert(
                        op_id,
                        PendingOp {
                            response_context,
                            source_tid: source,
                            source_pid,
                            kind,
                        },
                    )
                    .is_err()
                {
                    return Some(vec![build_error(source, ErrorCode::ResourceBusy)]);
                }
                return None;
            },
            Err(e) => return Some(vec![build_error(source, e)]),
        }
    }

    Some(super::long::handle_unlinkat(source, request))
}

pub(crate) fn handle_mkdirat_with_hostfs(
    response_context: ResponseContext,
    mut request: MakeDirectoryAtRequest,
    pending: &mut PendingQueue,
) -> Option<Vec<Message>> {
    let source_pid: ProcessIdentifier = response_context.source_pid();
    let source: ThreadIdentifier = response_context.source_tid();
    request.mode = ::vfs::fd::vfs_apply_umask(request.mode);
    let Some(resolved) = vfs_resolve_path(request.dirfd, &request.pathname) else {
        return Some(vec![build_error(source, ErrorCode::InvalidArgument)]);
    };
    let final_path: &str = &resolved;

    if hostfs::is_hostfs_path(final_path) {
        if !pending.has_capacity() {
            return Some(vec![build_error(source, ErrorCode::ResourceBusy)]);
        }
        let op_id: ::hostfs_api::OperationId = pending.alloc_op_id();
        match hostfs::send_mkdir_request(final_path, request.mode, op_id) {
            Ok(()) => {
                if pending
                    .insert(
                        op_id,
                        PendingOp {
                            response_context,
                            source_tid: source,
                            source_pid,
                            kind: PendingOpKind::Mkdir,
                        },
                    )
                    .is_err()
                {
                    return Some(vec![build_error(source, ErrorCode::ResourceBusy)]);
                }
                return None;
            },
            Err(e) => return Some(vec![build_error(source, e)]),
        }
    }

    Some(super::long::handle_mkdirat(source, request))
}

pub(crate) fn handle_symlinkat_with_hostfs(
    response_context: ResponseContext,
    request: SymbolicLinkAtRequest,
    pending: &mut PendingQueue,
) -> Option<Vec<Message>> {
    let source_pid: ProcessIdentifier = response_context.source_pid();
    let source: ThreadIdentifier = response_context.source_tid();
    // Routing key is `linkpath` (where the symlink will live). `target` is an opaque
    // string stored verbatim by the host and intentionally not consulted here.
    let Some(resolved) = vfs_resolve_path(request.dirfd, &request.linkpath) else {
        return Some(vec![build_error(source, ErrorCode::InvalidArgument)]);
    };
    let final_link: &str = &resolved;

    if hostfs::is_hostfs_path(final_link) {
        if !pending.has_capacity() {
            return Some(vec![build_error(source, ErrorCode::ResourceBusy)]);
        }
        let op_id: ::hostfs_api::OperationId = pending.alloc_op_id();
        match hostfs::send_symlink_request(&request.target, final_link, op_id) {
            Ok(()) => {
                if pending
                    .insert(
                        op_id,
                        PendingOp {
                            response_context,
                            source_tid: source,
                            source_pid,
                            kind: PendingOpKind::Symlink,
                        },
                    )
                    .is_err()
                {
                    return Some(vec![build_error(source, ErrorCode::ResourceBusy)]);
                }
                return None;
            },
            Err(e) => return Some(vec![build_error(source, e)]),
        }
    }

    Some(super::long::handle_symlinkat(source, request))
}

pub(crate) fn handle_readlinkat_with_hostfs(
    response_context: ResponseContext,
    request: ReadLinkAtRequest,
    pending: &mut PendingQueue,
) -> Option<Vec<Message>> {
    let source_pid: ProcessIdentifier = response_context.source_pid();
    let source: ThreadIdentifier = response_context.source_tid();
    let Some(resolved) = vfs_resolve_path(request.dirfd, &request.path) else {
        return Some(vec![build_error(source, ErrorCode::InvalidArgument)]);
    };
    let final_path: &str = &resolved;

    if hostfs::is_hostfs_path(final_path) {
        if !pending.has_capacity() {
            return Some(vec![build_error(source, ErrorCode::ResourceBusy)]);
        }
        let op_id: ::hostfs_api::OperationId = pending.alloc_op_id();
        let bufsiz: usize = request.bufsiz;
        match hostfs::send_readlink_request(final_path, op_id) {
            Ok(()) => {
                if pending
                    .insert(
                        op_id,
                        PendingOp {
                            response_context,
                            source_tid: source,
                            source_pid,
                            kind: PendingOpKind::Readlink { bufsiz },
                        },
                    )
                    .is_err()
                {
                    return Some(vec![build_error(source, ErrorCode::ResourceBusy)]);
                }
                return None;
            },
            Err(e) => return Some(vec![build_error(source, e)]),
        }
    }

    Some(super::long::handle_readlinkat(source, request))
}
