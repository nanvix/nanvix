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
//! (38) bytes per request, and write operations to
//! [`MAX_INLINE_WRITE_DATA`](::hostfs_api::MAX_INLINE_WRITE_DATA) (24) bytes. Larger
//! requests are silently clamped. Callers (the guest VFS layer) must handle short
//! reads/writes and issue additional requests for the remainder.
//!
//! When forwarding to hostfsd, these handlers send the IKC request and push a
//! [`PendingOp`] onto the pending queue. They return `None` to indicate that no
//! immediate response should be sent — the main event loop will complete the
//! operation when the IKC response arrives.

extern crate alloc;

use crate::{
    error::build_error,
    hostfs,
    pending::{
        PendingOp,
        PendingOpKind,
        PendingQueue,
    },
};
use ::sys::{
    error::ErrorCode,
    ipc::Message,
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::syscall::{
    unistd::message::{
        CloseRequest,
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
    source: ThreadIdentifier,
    msg: SystemCallMessage,
    pending: &mut PendingQueue,
) -> Option<Message> {
    let req: CloseRequest = CloseRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;

    if let Some(remote_fd) = ::vfs::fd::vfs_hostfs_remote_fd(fd) {
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
                    source_tid: source,
                    source_pid: None,
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

    Some(super::short::handle_close(source, msg))
}

pub(crate) fn handle_seek_with_hostfs(
    source: ThreadIdentifier,
    msg: SystemCallMessage,
    pending: &mut PendingQueue,
) -> Option<Message> {
    let req: SeekRequest = SeekRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;

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
                    source_tid: source,
                    source_pid: None,
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
    source: ThreadIdentifier,
    msg: SystemCallMessage,
    pending: &mut PendingQueue,
) -> Option<Message> {
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
                    source_tid: source,
                    source_pid: None,
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
    source: ThreadIdentifier,
    msg: SystemCallMessage,
    pending: &mut PendingQueue,
) -> Option<Message> {
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
                    source_tid: source,
                    source_pid: None,
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
    source: ThreadIdentifier,
    msg: SystemCallMessage,
    pending: &mut PendingQueue,
) -> Option<Vec<Message>> {
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
                    source_tid: source,
                    source_pid: None,
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
    source: ThreadIdentifier,
    request: FileStatAtRequest,
    _pending: &mut PendingQueue,
) -> Option<Vec<Message>> {
    let resolved: Option<alloc::string::String> =
        ::vfs::fd::vfs_resolve_path(request.dirfd, &request.path);
    let final_path: &str = match &resolved {
        Some(p) => p.as_str(),
        None => &request.path,
    };

    if hostfs::is_hostfs_path(final_path) {
        // For path-based stat on hostfs, we need an FD. Open + stat + close sequence
        // would be complex. Instead, use a temporary open to get a remote FD, then stat.
        // However, the simpler approach is to open the path first and call fstat.
        // Since we don't have a "stat-by-path" IKC that returns without an FD open,
        // we use the existing open + stat. But that's two async ops.
        //
        // Alternative: the hostfsd stat handler uses the FD's stored path, so we need
        // a valid remote FD. We can open with O_RDONLY, stat, then close.
        // But that requires chaining three async operations, which is complex.
        //
        // For now, open the file to get a remote_fd, then send stat. We'll handle the
        // response by returning stat and cleaning up the temp FD.
        //
        // Actually, re-reading the hostfsd code: `handle_stat` takes an FD and looks up
        // the path from its FD table. So we DO need to open first to stat.
        //
        // Simpler: just open O_RDONLY, and in the open completion we'd need to stat...
        // That's overly complex for this handler.
        //
        // The practical solution: for fstatat on hostfs paths, we first try to open
        // the path. If the user already has an FD open for this path, that's ideal.
        // But since we need a remote FD to call stat, and we don't want to chain ops,
        // fall back to returning a synthetic stat for now.
        //
        // TODO(#hostfs-statat): implement path-based stat as a new hostfs operation that
        // does not require a pre-opened FD.
        //
        // For now, return a minimal stat that at least allows path existence checks.
        // We attempt to open and immediately use the resulting info.
        // Actually, the simplest approach that works: return OperationNotSupported so
        // the caller knows to use fstat on an already-opened FD instead.
        return Some(vec![build_error(source, ErrorCode::OperationNotSupported)]);
    }

    Some(super::long::handle_fstatat(source, request))
}

pub(crate) fn handle_read_with_hostfs(
    source_pid: ProcessIdentifier,
    source_tid: ThreadIdentifier,
    msg: SystemCallMessage,
    pending: &mut PendingQueue,
) -> Option<Message> {
    let req: ReadRequest = ReadRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;

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
                    source_tid,
                    source_pid: Some(source_pid),
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
    source_pid: ProcessIdentifier,
    source_tid: ThreadIdentifier,
    msg: SystemCallMessage,
    pending: &mut PendingQueue,
) -> Option<Message> {
    let req: WriteRequest = WriteRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;

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
                            source_tid,
                            source_pid: Some(source_pid),
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
};
use alloc::{
    vec,
    vec::Vec,
};

/// Handles getdents with hostfs awareness.
///
/// If the FD is backed by hostfs, returns `OperationNotSupported`. Directory listing
/// over hostfs requires multiple sequential IKC round-trips (one per entry), which
/// cannot be performed synchronously within the single-message getdents handler without
/// blocking the event loop.
///
/// # Known limitation
///
/// `getdents` on hostfs-backed file descriptors is not yet supported. This means that
/// `ls /mnt/` (or any readdir-based directory listing) will fail with
/// `OperationNotSupported` for paths served by hostfsd. Users must access individual
/// files by full path.
///
/// TODO(#hostfs-getdents): convert getdents into an async multi-step operation via the
/// pending queue so that `ls /mnt/` works from the guest.
pub(crate) fn handle_getdents_with_hostfs(
    source: ThreadIdentifier,
    msg: SystemCallMessage,
) -> Vec<Message> {
    let req: GetDirectoryEntriesRequest = GetDirectoryEntriesRequest::from_bytes(msg.payload);
    let fd: i32 = req.fd;
    if ::vfs::fd::is_hostfs_fd(fd) {
        ::syslog::warn!("getdents on hostfs fd {} not supported (use hostfs readdir protocol)", fd);
        return vec![build_error(source, ErrorCode::OperationNotSupported)];
    }
    super::long::handle_getdents(source, msg)
}

pub(crate) fn handle_openat_with_hostfs(
    source: ThreadIdentifier,
    request: OpenAtRequest,
    pending: &mut PendingQueue,
) -> Option<Vec<Message>> {
    let path: &str = &request.pathname;
    let resolved: Option<alloc::string::String> = ::vfs::fd::vfs_resolve_path(request.dirfd, path);
    let final_path: &str = match &resolved {
        Some(p) => p.as_str(),
        None => path,
    };

    if hostfs::is_hostfs_path(final_path) {
        if !pending.has_capacity() {
            return Some(vec![build_error(source, ErrorCode::ResourceBusy)]);
        }
        let op_id: ::hostfs_api::OperationId = pending.alloc_op_id();
        match hostfs::send_open_request(final_path, request.flags, op_id) {
            Ok(()) => {
                if pending
                    .insert(
                        op_id,
                        PendingOp {
                            source_tid: source,
                            source_pid: None,
                            kind: PendingOpKind::Open,
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
    source: ThreadIdentifier,
    request: RenameAtRequest,
    pending: &mut PendingQueue,
) -> Option<Vec<Message>> {
    let old_resolved: Option<alloc::string::String> =
        ::vfs::fd::vfs_resolve_path(request.olddirfd, &request.oldpath);
    let new_resolved: Option<alloc::string::String> =
        ::vfs::fd::vfs_resolve_path(request.newdirfd, &request.newpath);
    let old_final: &str = match &old_resolved {
        Some(p) => p.as_str(),
        None => &request.oldpath,
    };
    let new_final: &str = match &new_resolved {
        Some(p) => p.as_str(),
        None => &request.newpath,
    };

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
                            source_tid: source,
                            source_pid: None,
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
    source: ThreadIdentifier,
    request: UnlinkAtRequest,
    pending: &mut PendingQueue,
) -> Option<Vec<Message>> {
    let resolved: Option<alloc::string::String> =
        ::vfs::fd::vfs_resolve_path(request.dirfd, &request.pathname);
    let final_path: &str = match &resolved {
        Some(p) => p.as_str(),
        None => &request.pathname,
    };

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
                            source_tid: source,
                            source_pid: None,
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
    source: ThreadIdentifier,
    request: MakeDirectoryAtRequest,
    pending: &mut PendingQueue,
) -> Option<Vec<Message>> {
    let resolved: Option<alloc::string::String> =
        ::vfs::fd::vfs_resolve_path(request.dirfd, &request.pathname);
    let final_path: &str = match &resolved {
        Some(p) => p.as_str(),
        None => &request.pathname,
    };

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
                            source_tid: source,
                            source_pid: None,
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
