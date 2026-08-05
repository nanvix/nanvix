// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Pending hostfs operation tracking.
//!
//! When vfsd forwards a request to hostfsd via IKC, it cannot block waiting for
//! the response without stalling the entire daemon. Instead, the request is sent
//! non-blocking and a [`PendingOp`] record is stored. When the IKC response arrives
//! in the main event loop, the pending operation is completed and the result is
//! sent back to the original guest caller.
//!
//! # Preconditions
//!
//! Pending entries have no timeout. If `hostfs::enable()` is called without a
//! hostfsd worker actively servicing IKC requests, entries will accumulate
//! indefinitely and callers will deadlock waiting for responses. The mount handler
//! documents this precondition; see [`super::handler::mount_handler::handle_mount`].

extern crate alloc;

use crate::error::{
    build_error,
    ResponseContext,
};
use ::alloc::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    vec::Vec,
};
use ::core::time::Duration;
use ::hostfs_api::{
    OperationId,
    OperationIdAllocator,
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
    time::SystemTime,
};
use ::sysapi::{
    pthread::{
        PTHREAD_COND_INITIALIZER,
        PTHREAD_MUTEX_INITIALIZER,
    },
    sys_types::{
        pthread_cond_t,
        pthread_mutex_t,
    },
};

//==================================================================================================
// Pending Operation Descriptor
//==================================================================================================

/// Describes a hostfs operation that is waiting for an IKC response from hostfsd.
pub(crate) struct PendingOp {
    /// Exact response routing and correlation metadata for the original request.
    pub response_context: ResponseContext,
    /// Thread that initiated the request, used by payload builders and push rendezvous.
    pub source_tid: ThreadIdentifier,
    /// Process that initiated the request, used for VFS state and push rendezvous.
    pub source_pid: ProcessIdentifier,
    /// The kind of operation, which determines how to interpret the IKC response.
    pub kind: PendingOpKind,
}

/// The specific hostfs operation being awaited.
pub(crate) enum PendingOpKind {
    /// open() — response contains a remote FD; we allocate a local hostfs FD.
    Open {
        /// Absolute path that was opened (stored so `HostFsHandle` can resolve relative paths).
        path: alloc::string::String,
    },
    /// close() — response is a status code; local FD has already been released.
    Close,
    /// read() — response contains inline data; push it to the caller.
    Read {
        /// Number of bytes the caller requested.
        count: usize,
        /// Guest descriptor whose shared offset advances after delivery.
        fd: i32,
        /// Remote descriptor used to reject guest descriptor reuse.
        remote_fd: i32,
        /// Virtual offset used for the positional host read.
        offset: i64,
        /// Completed host response retained until the caller's pull is ready.
        response: Option<::hostfs_api::ReadResponse>,
        /// Error metadata waiting for its empty transfer to be registered.
        delivery_error: Option<ErrorCode>,
        /// Number of timed delivery retries already attempted.
        delivery_retries: u8,
    },
    /// write() — response contains bytes_written count.
    Write {
        /// Guest descriptor whose shared offset advances after completion.
        fd: i32,
        /// Remote descriptor used to reject guest descriptor reuse.
        remote_fd: i32,
        /// Virtual offset used for the positional host write.
        offset: i64,
    },
    /// lseek() — response contains the new offset.
    Seek {
        /// Guest descriptor whose virtual offset is updated.
        fd: i32,
        /// Remote descriptor used to reject guest descriptor reuse.
        remote_fd: i32,
        /// Virtual offset observed when the request was issued.
        previous_offset: i64,
    },
    /// fsync/flush — response is a status code.
    Flush,
    /// ftruncate — response is a status code.
    Truncate,
    /// mkdir — response is a status code.
    Mkdir,
    /// rmdir — response is a status code.
    Rmdir,
    /// unlink — response is a status code.
    Unlink,
    /// rename — response is a status code.
    Rename,
    /// stat/fstat — response contains size, mode, is_dir.
    Stat,
    /// symlink — response is a status code.
    Symlink,
    /// readlink — response contains the link target bytes.
    Readlink {
        /// Caller-supplied buffer size; response will be truncated to this length.
        bufsiz: usize,
    },
    /// lstat — path-based stat that does not follow the final symbolic link.
    Lstat,
    /// Path-based stat that follows the final symbolic link (default `stat(2)` semantics).
    /// Shares the `lstat` response wire format and completion path.
    PathStat,
    /// getdents — directory listing over hostfs.
    ///
    /// hostfsd returns one entry per IKC round-trip, so a single guest `getdents`
    /// call is served by an async sweep that issues repeated readdir requests under
    /// the same op_id until `target_count` entries are collected or the directory is
    /// exhausted. The accumulated entries are buffered here across round-trips.
    Getdents {
        /// Remote (hostfsd) directory file descriptor.
        remote_fd: i32,
        /// Guest-visible FD, used to persist the iteration cursor after completion.
        guest_fd: i32,
        /// Offset of the next directory entry to request.
        next_offset: u32,
        /// Number of entries the guest asked for in this `getdents` call.
        target_count: usize,
        /// Entries collected so far in this sweep.
        entries: ::alloc::vec::Vec<::sysapi::dirent::posix_dent>,
    },
}

fn pending_io_remote_fd(kind: &PendingOpKind) -> Option<i32> {
    match kind {
        PendingOpKind::Read { remote_fd, .. }
        | PendingOpKind::Write { remote_fd, .. }
        | PendingOpKind::Seek { remote_fd, .. } => Some(*remote_fd),
        _ => None,
    }
}

//==================================================================================================
// Pending Operation Queue
//==================================================================================================

/// Maximum number of pending operations before new requests are rejected.
///
/// This prevents unbounded growth if hostfsd is unavailable or unresponsive.
const MAX_PENDING_OPS: usize = 64;

/// Maximum number of timed attempts to deliver a completed HostFS read.
const MAX_READ_DELIVERY_RETRIES: u8 = 6;

/// Timeout for the first completed-read delivery retry.
const READ_DELIVERY_RETRY_BASE_DELAY: Duration = Duration::from_millis(5);

/// Completed HostFS reads waiting for their next delivery attempt.
static READ_RETRY_SCHEDULE: ::spin::Mutex<BTreeMap<OperationId, SystemTime>> =
    ::spin::Mutex::new(BTreeMap::new());

/// Wakes the retry scheduler when a new deadline is inserted.
static READ_RETRY_CONDITION: pthread_cond_t = PTHREAD_COND_INITIALIZER;

/// Coordinates deadline insertion with the scheduler's condition wait.
static mut READ_RETRY_MUTEX: pthread_mutex_t = PTHREAD_MUTEX_INITIALIZER;

//==================================================================================================
// Synthetic stat(2) Constants
//==================================================================================================
//
// Hostfsd does not forward several `stat`/`lstat` fields from the host (timestamps,
// device id, inode, link count). The constants below are the synthetic values used
// to populate those fields so both completion paths report identical, deterministic
// metadata.

/// Fixed timestamp (2024-01-01T00:00:00Z) used for `st_atim`/`st_mtim`/`st_ctim`.
///
/// Keeps stat output stable across runs and hosts; tooling that only cares about
/// ordering or equality continues to work.
const STAT_FIXED_EPOCH: i64 = 1_704_067_200;

/// Conventional Unix block size reported as `st_blksize`.
///
/// Matches what guest userland and libc helpers (e.g., `st_blksize`-based I/O sizing)
/// expect from a regular filesystem.
const STAT_BLOCK_SIZE: i64 = 4096;

/// POSIX-defined unit (in bytes) used to convert `st_size` into `st_blocks`.
const STAT_SECTOR_SIZE: u64 = 512;

/// Synthetic device id reported as `st_dev` for hostfs entries.
///
/// Distinct from ramfs (`1`) so guest tooling can tell the two filesystems apart;
/// hostfsd does not expose the host's real `st_dev`.
const STAT_HOSTFS_DEV: u64 = 2;

/// Synthetic inode number reported as `st_ino`.
///
/// Inode numbers are not tracked by hostfsd; a constant keeps the field valid
/// without implying any cross-call identity (callers must not key caches on it).
const STAT_SYNTHETIC_INO: u64 = 1;

/// `st_nlink` value for directories (self + `.`).
const STAT_NLINK_DIR: u64 = 2;

/// `st_nlink` value for non-directory entries (hostfsd does not track hardlinks).
const STAT_NLINK_FILE: u64 = 1;

/// Map of pending hostfs operations keyed by operation identifier.
///
/// Each outgoing IKC request carries a unique `op_id` (assigned by [`alloc_op_id`])
/// that hostfsd echoes back in its response. The main event loop extracts the `op_id`
/// from the response and looks up the corresponding [`PendingOp`] to complete it.
///
/// # Limitations
///
/// There is currently no timeout mechanism for pending operations. If the hostfsd worker
/// crashes or the IKC channel is severed, callers will remain blocked indefinitely.
/// TODO(#hostfs-timeout): implement a tick-based watchdog that drains stale entries after
/// a configurable deadline (e.g., 5 seconds without a response).
pub(crate) struct PendingQueue {
    ops: BTreeMap<OperationId, PendingOp>,
    abandoned_ops: BTreeSet<OperationId>,
    abandoned_opens: BTreeSet<OperationId>,
    abandoned_io: BTreeMap<OperationId, i32>,
    read_retries: BTreeSet<OperationId>,
    id_alloc: OperationIdAllocator,
}

impl PendingQueue {
    /// Creates an empty pending queue.
    pub fn new() -> Self {
        Self {
            ops: BTreeMap::new(),
            abandoned_ops: BTreeSet::new(),
            abandoned_opens: BTreeSet::new(),
            abandoned_io: BTreeMap::new(),
            read_retries: BTreeSet::new(),
            id_alloc: OperationIdAllocator::new(),
        }
    }

    /// Returns whether an offset-sensitive operation is active for `remote_fd`.
    pub fn has_active_io(&self, remote_fd: i32) -> bool {
        self.ops
            .values()
            .any(|op| pending_io_remote_fd(&op.kind) == Some(remote_fd))
            || self
                .abandoned_io
                .values()
                .any(|active| *active == remote_fd)
    }

    /// Allocates the next unique operation identifier.
    ///
    /// The returned ID is guaranteed not to collide with any currently pending operation.
    /// Callers should use this ID when sending the IKC request so that the response can
    /// be matched back via [`remove`](Self::remove).
    pub fn alloc_op_id(&mut self) -> OperationId {
        self.id_alloc
            .alloc(|id| self.ops.contains_key(id) || self.abandoned_ops.contains(id))
    }

    /// Inserts a pending operation under the given operation identifier.
    ///
    /// Returns `Err(ErrorCode::ResourceBusy)` if the queue is full, so the caller
    /// can propagate the error without crashing vfsd.
    pub fn insert(&mut self, op_id: OperationId, op: PendingOp) -> Result<(), ErrorCode> {
        if self.ops.len() + self.abandoned_ops.len() >= MAX_PENDING_OPS {
            return Err(ErrorCode::ResourceBusy);
        }
        self.ops.insert(op_id, op);
        Ok(())
    }

    /// Returns `true` if the queue has capacity for at least one more operation.
    ///
    /// Callers should check this BEFORE sending an IKC request to avoid orphaned
    /// responses when the queue is full.
    pub fn has_capacity(&self) -> bool {
        self.ops.len() + self.abandoned_ops.len() < MAX_PENDING_OPS
    }

    /// Removes and returns the pending operation associated with the given `op_id`.
    pub fn remove(&mut self, op_id: OperationId) -> Option<PendingOp> {
        self.read_retries.remove(&op_id);
        cancel_scheduled_read_retry(op_id);
        self.ops.remove(&op_id)
    }

    /// Returns a mutable reference to the pending operation for the given `op_id`.
    ///
    /// Used by multi-round-trip operations (e.g. getdents) that mutate buffered state
    /// in place across IKC responses without removing the op until the sweep completes.
    pub fn get_mut(&mut self, op_id: OperationId) -> Option<&mut PendingOp> {
        self.ops.get_mut(&op_id)
    }

    /// Retains a completed read and schedules another nonblocking delivery attempt.
    pub fn defer_read_delivery(&mut self, op_id: OperationId, op: PendingOp) {
        let delay: Duration = match &op.kind {
            PendingOpKind::Read {
                delivery_retries, ..
            } => {
                READ_DELIVERY_RETRY_BASE_DELAY.saturating_mul(1u32 << u32::from(*delivery_retries))
            },
            _ => {
                ::syslog::error!(
                    "cannot defer delivery for a non-read HostFS operation (op_id={})",
                    op_id
                );
                cancel_pending_op(op, ErrorCode::IoErr);
                return;
            },
        };
        self.ops.insert(op_id, op);
        if !self.read_retries.insert(op_id) {
            return;
        }
        if let Err(error) = schedule_read_retry(op_id, delay) {
            self.read_retries.remove(&op_id);
            if let Some(op) = self.ops.remove(&op_id) {
                ::syslog::error!(
                    "failed to schedule HostFS read retry (op_id={}, error={:?})",
                    op_id,
                    error
                );
                cancel_pending_op(op, ErrorCode::IoErr);
            }
        }
    }

    /// Retries one retained HostFS read delivery.
    pub fn retry_read_delivery(&mut self, op_id: OperationId) {
        self.read_retries.remove(&op_id);
        let Some(mut op): Option<PendingOp> = self.ops.remove(&op_id) else {
            return;
        };
        ::vfs::fd::set_current_process(op.source_pid);
        let retries_exhausted: bool = match &mut op.kind {
            PendingOpKind::Read {
                delivery_retries, ..
            } if *delivery_retries < MAX_READ_DELIVERY_RETRIES => {
                *delivery_retries += 1;
                *delivery_retries == MAX_READ_DELIVERY_RETRIES
            },
            PendingOpKind::Read { .. } => true,
            _ => {
                ::syslog::error!(
                    "HostFS read retry references a non-read operation (op_id={})",
                    op_id
                );
                cancel_pending_op(op, ErrorCode::IoErr);
                return;
            },
        };
        if complete_read(&op) {
            return;
        }
        if retries_exhausted {
            ::syslog::warn!(
                "HostFS read delivery retries exhausted (op_id={}, pid={:?}, tid={:?})",
                op_id,
                op.source_pid,
                op.source_tid
            );
            if let PendingOpKind::Read { delivery_error, .. } = &mut op.kind {
                *delivery_error = Some(ErrorCode::OperationTimedOut);
            }
            if complete_read(&op) {
                return;
            }
        }
        self.defer_read_delivery(op_id, op);
    }

    /// Cancels one exact pending hostfs read and retains its ID until the late response drains.
    pub fn cancel_read_request(
        &mut self,
        pid: ProcessIdentifier,
        tid: ThreadIdentifier,
        request_id: ::sys::ipc::RequestIdentifier,
    ) -> bool {
        let op_id: Option<OperationId> = self.ops.iter().find_map(|(op_id, op)| {
            (op.source_pid == pid
                && op.source_tid == tid
                && op.response_context.request_id() == request_id
                && matches!(op.kind, PendingOpKind::Read { .. }))
            .then_some(*op_id)
        });
        if let Some(op_id) = op_id {
            self.read_retries.remove(&op_id);
            cancel_scheduled_read_retry(op_id);
            if let Some(op) = self.ops.remove(&op_id) {
                let response_pending: bool =
                    matches!(&op.kind, PendingOpKind::Read { response: None, .. });
                if response_pending {
                    if let Some(remote_fd) = pending_io_remote_fd(&op.kind) {
                        self.abandoned_io.insert(op_id, remote_fd);
                    }
                    self.abandoned_ops.insert(op_id);
                }
            }
            true
        } else {
            match ::sys::kcall::ipc::__kcall_cancel_tagged_deferred_push(pid, tid, request_id) {
                Ok(()) => true,
                Err(error) if error.code == ErrorCode::NoSuchEntry => false,
                Err(error) => {
                    ::syslog::warn!(
                        "failed to cancel deferred HostFS read error (pid={:?}, tid={:?}, \
                         error={:?})",
                        pid,
                        tid,
                        error
                    );
                    false
                },
            }
        }
    }

    /// Removes pending operations owned by `pid`, retaining IDs until late responses drain.
    pub fn purge_pid(&mut self, pid: ProcessIdentifier) {
        let op_ids: ::alloc::vec::Vec<OperationId> = self
            .ops
            .iter()
            .filter_map(|(op_id, op)| (op.source_pid == pid).then_some(*op_id))
            .collect();
        for op_id in op_ids {
            self.read_retries.remove(&op_id);
            cancel_scheduled_read_retry(op_id);
            if let Some(op) = self.ops.remove(&op_id) {
                let response_pending: bool = !matches!(
                    &op.kind,
                    PendingOpKind::Read {
                        response: Some(_),
                        ..
                    }
                );
                if response_pending {
                    self.abandoned_ops.insert(op_id);
                    if matches!(&op.kind, PendingOpKind::Open { .. }) {
                        self.abandoned_opens.insert(op_id);
                    }
                    if let Some(remote_fd) = pending_io_remote_fd(&op.kind) {
                        self.abandoned_io.insert(op_id, remote_fd);
                    }
                }
            }
        }
    }

    /// Handles a late response for an operation abandoned by cancellation, exit, or exec.
    ///
    /// A successful abandoned open is closed remotely; other responses are discarded.
    pub fn complete_abandoned_operation(
        &mut self,
        op_id: OperationId,
        response_payload: &[u8; Message::PAYLOAD_SIZE],
    ) -> bool {
        if !self.abandoned_ops.remove(&op_id) {
            return false;
        }
        self.abandoned_io.remove(&op_id);
        if !self.abandoned_opens.remove(&op_id) {
            return true;
        }

        let header_raw: u16 = u16::from_ne_bytes([response_payload[0], response_payload[1]]);
        if ::syscall::SystemCallMessageHeader::try_from(header_raw)
            != Ok(::syscall::SystemCallMessageHeader::HostFsOpenResponse)
        {
            ::syslog::warn!(
                "late abandoned hostfs open has invalid response header (op_id={})",
                op_id
            );
            return true;
        }

        let response: ::hostfs_api::OpenResponse =
            ::hostfs_api::OpenResponse::decode(response_payload);
        let remote_fd: i32 = response.fd;
        if remote_fd >= 0 {
            if let Err(error) =
                crate::hostfs::send_close_request(remote_fd, OperationId::FIRE_AND_FORGET)
            {
                ::syslog::warn!(
                    "failed to close late abandoned hostfs open (op_id={}, remote_fd={}, \
                     error={:?})",
                    op_id,
                    remote_fd,
                    error
                );
            }
        }
        true
    }

    /// Discards a completed multipart response for an abandoned operation.
    pub fn discard_abandoned_operation(&mut self, op_id: OperationId) -> bool {
        if !self.abandoned_ops.remove(&op_id) {
            return false;
        }
        self.abandoned_opens.remove(&op_id);
        self.abandoned_io.remove(&op_id);
        true
    }

    /// Returns true if there are no pending operations.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Drains all pending operations, sending an error response to each waiting caller.
    ///
    /// This is used for recovery scenarios (e.g., channel loss) where all pending
    /// callers must be unblocked with an error.
    #[allow(dead_code)]
    pub fn drain_with_error(&mut self) {
        self.read_retries.clear();
        READ_RETRY_SCHEDULE.lock().clear();
        for (_, op) in core::mem::take(&mut self.ops) {
            ::syslog::error!(
                "hostfs pending queue drain: failing pending op for tid={:?}",
                op.source_tid
            );
            cancel_pending_op(op, ErrorCode::IoErr);
        }
    }
}

/// Sends an error response to a pending op's caller without consulting any IKC
/// payload. Intended for recovery paths (e.g., a long-response stream is discarded
/// due to assembler desync) where the op must be cancelled rather than completed.
pub(crate) fn cancel_pending_op(op: PendingOp, code: ErrorCode) {
    if matches!(op.kind, PendingOpKind::Read { .. }) {
        if let Err(error) = fail_pending_read(&op, code) {
            if !matches!(error.code, ErrorCode::NoSuchEntry | ErrorCode::NoSuchProcess) {
                ::syslog::warn!(
                    "failed to deliver HostFS read cancellation (pid={:?}, tid={:?}, error={:?})",
                    op.source_pid,
                    op.source_tid,
                    error
                );
            }
        }
        return;
    }
    op.response_context.send(&build_error(op.source_tid, code));
}

/// Builds immediate read-error metadata only after guaranteeing its empty bulk transfer.
pub(crate) fn prepare_read_error(
    response_context: ResponseContext,
    code: ErrorCode,
) -> Option<Message> {
    match register_deferred_empty(response_context) {
        Ok(()) => Some(build_error(response_context.source_tid(), code)),
        Err(error) if matches!(error.code, ErrorCode::NoSuchEntry | ErrorCode::NoSuchProcess) => {
            None
        },
        Err(error) => {
            ::syslog::error!(
                "failed to prepare deferred read error (pid={:?}, tid={:?}, error={:?})",
                response_context.source_pid(),
                response_context.source_tid(),
                error
            );
            None
        },
    }
}

fn register_deferred_empty(response_context: ResponseContext) -> Result<(), ::sys::error::Error> {
    match ::sys::kcall::ipc::__kcall_push_tagged_deferred(
        response_context.source_pid(),
        response_context.source_tid(),
        response_context.request_id(),
    ) {
        Ok(()) => Ok(()),
        Err(error) if error.code == ErrorCode::OperationAlreadyInProgress => Ok(()),
        Err(error) => Err(error),
    }
}

/// Registers an empty transfer for a current or future pull, then sends its error metadata.
fn fail_pending_read(op: &PendingOp, code: ErrorCode) -> Result<(), ::sys::error::Error> {
    register_deferred_empty(op.response_context)?;
    op.response_context.send(&build_error(op.source_tid, code));
    Ok(())
}

//==================================================================================================
// Response Completion
//==================================================================================================

/// Completes a pending hostfs operation given the IKC response payload.
///
/// Builds and sends the appropriate response message to the guest caller.
/// Validates that the response header matches the expected operation to detect
/// protocol violations. On mismatch, fails only the affected operation.
pub(crate) fn complete_pending_op(
    mut pending: PendingOp,
    response_payload: &[u8; Message::PAYLOAD_SIZE],
) -> Option<PendingOp> {
    let response_context: ResponseContext = pending.response_context;
    // Bind the VFS to the requesting process so that descriptor allocation (e.g. for a completed
    // open) and directory-cursor updates land in its per-process state. `source_pid` was copied
    // from the kernel-attested `message.source.pid` when the request was dispatched, so it remains
    // correct even when the caller's thread identifier differs from its process identifier. vfsd
    // being single-threaded is what makes mutating this global current-process selector race-free.
    //
    // The caller is guaranteed to still be registered here: guest syscalls are synchronous, so it
    // stays blocked awaiting this very response and cannot exit, and the only involuntary
    // termination path (memd killing a faulting process) cannot target a process parked in a
    // syscall. Completion therefore never resurrects an exited process — which would re-create an
    // empty placeholder and leak any host handle this op allocates (e.g. a completed open).
    let current_pid: ProcessIdentifier = pending.source_pid;
    ::vfs::fd::set_current_process(current_pid);

    // Validate that the response header matches the expected operation kind.
    if !validate_response_header(&pending.kind, response_payload) {
        ::syslog::error!("hostfs pending op: response header does not match expected operation");
        if let PendingOpKind::Read { delivery_error, .. } = &mut pending.kind {
            *delivery_error = Some(ErrorCode::IoErr);
            return if complete_read(&pending) {
                None
            } else {
                Some(pending)
            };
        }
        response_context.send(&build_error(pending.source_tid, ErrorCode::IoErr));
        return None;
    }

    if let PendingOpKind::Read { response, .. } = &mut pending.kind {
        *response = Some(::hostfs_api::ReadResponse::decode(response_payload));
        return if complete_read(&pending) {
            None
        } else {
            Some(pending)
        };
    }

    match pending.kind {
        PendingOpKind::Open { path } => complete_open(response_context, response_payload, path),
        PendingOpKind::Close => complete_close(response_context, response_payload),
        PendingOpKind::Read { .. } => {
            ::syslog::error!("HostFS read reached non-read completion path");
            response_context.send(&build_error(pending.source_tid, ErrorCode::IoErr));
        },
        PendingOpKind::Write {
            fd,
            remote_fd,
            offset,
        } => complete_write(response_context, fd, remote_fd, offset, response_payload),
        PendingOpKind::Seek {
            fd,
            remote_fd,
            previous_offset,
        } => complete_seek(response_context, fd, remote_fd, previous_offset, response_payload),
        PendingOpKind::Flush => complete_status(response_context, response_payload, OpGroup::Flush),
        PendingOpKind::Truncate => {
            complete_status(response_context, response_payload, OpGroup::Truncate)
        },
        PendingOpKind::Mkdir => complete_status(response_context, response_payload, OpGroup::Mkdir),
        PendingOpKind::Rmdir => complete_status(response_context, response_payload, OpGroup::Rmdir),
        PendingOpKind::Unlink => {
            complete_status(response_context, response_payload, OpGroup::Unlink)
        },
        PendingOpKind::Rename => {
            complete_status(response_context, response_payload, OpGroup::Rename)
        },
        PendingOpKind::Stat => complete_stat(response_context, response_payload),
        PendingOpKind::Symlink => {
            complete_status(response_context, response_payload, OpGroup::Symlink)
        },
        PendingOpKind::Readlink { bufsiz } => {
            complete_readlink(response_context, response_payload, bufsiz)
        },
        PendingOpKind::Lstat => complete_lstat(response_context, response_payload),
        PendingOpKind::PathStat => complete_lstat(response_context, response_payload),
        PendingOpKind::Getdents { .. } => {
            // Getdents sweeps are driven entirely by the main event loop, which keeps
            // the op buffered across round-trips and finalizes it via `finish_getdents`.
            // Reaching here means a single-shot completion was attempted for a getdents
            // op, which is a logic error.
            ::syslog::error!("getdents pending op routed to single-shot completion");
            response_context.send(&build_error(pending.source_tid, ErrorCode::IoErr));
        },
    }
    None
}

/// Outcome of advancing a getdents sweep with one hostfs readdir response.
pub(crate) enum GetdentsStep {
    /// More entries are required; issue another readdir request for `remote_fd` at `offset`.
    Continue {
        /// Remote (hostfsd) directory file descriptor to query.
        remote_fd: i32,
        /// Offset of the next directory entry to request.
        offset: u32,
    },
    /// The sweep is complete; call [`finish_getdents`] to send the response.
    Done,
    /// The host operation failed; cancel the sweep with this error.
    Failed(ErrorCode),
}

/// Appends one directory entry to a getdents sweep and reports whether the sweep is done.
///
/// Shared by the inline ([`step_getdents`]) and multi-part long-name readdir paths.
/// `name` is the raw entry name (already extracted from whichever wire form delivered
/// it). Names longer than the guest `NAME_MAX` cannot be represented in a `posix_dent`,
/// so they are clamped (and a warning is logged) rather than corrupting the buffer.
///
/// # Panics
///
/// Panics if `op` is not a [`PendingOpKind::Getdents`].
pub(crate) fn push_getdents_entry(op: &mut PendingOp, name: &[u8], is_dir: bool) -> GetdentsStep {
    use ::sysapi::{
        dirent::{
            dirent_file_type,
            posix_dent,
        },
        limits::NAME_MAX,
    };

    let PendingOpKind::Getdents {
        remote_fd,
        next_offset,
        target_count,
        entries,
        ..
    } = &mut op.kind
    else {
        unreachable!("push_getdents_entry invoked with non-Getdents pending op");
    };

    let copy_len: usize = name.len().min(NAME_MAX);
    if name.len() > NAME_MAX {
        ::syslog::warn!(
            "getdents: directory entry name exceeds NAME_MAX ({} > {}), clamping",
            name.len(),
            NAME_MAX
        );
    }

    let mut dent: posix_dent = posix_dent {
        // hostfs has no stable inode numbers; use a synthetic 1-based index.
        d_ino: (*next_offset as u64) + 1,
        d_reclen: core::mem::size_of::<posix_dent>() as u16,
        d_type: if is_dir {
            dirent_file_type::DT_DIR
        } else {
            dirent_file_type::DT_REG
        },
        ..posix_dent::default()
    };
    dent.d_name[..copy_len].copy_from_slice(&name[..copy_len]);
    dent.d_name[copy_len] = 0;
    entries.push(dent);

    *next_offset += 1;

    if entries.len() >= *target_count {
        GetdentsStep::Done
    } else {
        GetdentsStep::Continue {
            remote_fd: *remote_fd,
            offset: *next_offset,
        }
    }
}

/// Advances a getdents sweep with a single inline hostfs readdir response.
///
/// Decodes one directory entry from `response_payload` and appends it to the op's
/// buffer. A zero-length entry name marks end-of-directory. Returns whether another
/// round-trip is required or the sweep is finished. Long entry names are delivered via
/// a multi-part stream and handled separately (see [`push_getdents_entry`]).
///
/// # Panics
///
/// Panics if `op` is not a [`PendingOpKind::Getdents`]; callers must route only getdents
/// ops here.
pub(crate) fn step_getdents(
    op: &mut PendingOp,
    response_payload: &[u8; Message::PAYLOAD_SIZE],
) -> GetdentsStep {
    let entry: ::hostfs_api::ReadDirEntry = ::hostfs_api::ReadDirEntry::decode(response_payload);

    if entry.status < 0 {
        return GetdentsStep::Failed(hostfs_error_to_code(entry.status));
    }

    // A zero-length name marks the end of the directory.
    if entry.name_len == 0 {
        return GetdentsStep::Done;
    }

    let name_len: usize = (entry.name_len as usize).min(::hostfs_api::MAX_DIR_ENTRY_NAME_LEN);
    push_getdents_entry(op, &entry.name[..name_len], entry.is_dir != 0)
}

/// Finalizes a getdents sweep: persists the directory cursor and sends the response.
///
/// # Panics
///
/// Panics if `op` is not a [`PendingOpKind::Getdents`].
pub(crate) fn finish_getdents(op: PendingOp) {
    use ::syscall::{
        dirent::message::GetDirectoryEntriesResponse,
        message::MessagePartitioner,
    };

    let response_context: ResponseContext = op.response_context;
    let source_tid: ThreadIdentifier = op.source_tid;
    let PendingOpKind::Getdents {
        remote_fd,
        guest_fd,
        next_offset,
        entries,
        ..
    } = op.kind
    else {
        unreachable!("finish_getdents invoked with non-Getdents pending op");
    };

    ::vfs::fd::set_current_process(op.source_pid);

    // Persist the iteration cursor so the next getdents call resumes where this left off.
    // Guard against FD reuse: if the guest FD was closed and re-bound to a different host
    // file while this sweep was in flight, do not clobber the unrelated handle's cursor.
    if ::vfs::fd::vfs_hostfs_remote_fd(guest_fd) == Some(remote_fd) {
        ::vfs::fd::vfs_hostfs_set_readdir_offset(guest_fd, next_offset);
    }

    let response: GetDirectoryEntriesResponse = GetDirectoryEntriesResponse::new(entries);
    match response.into_parts(source_tid, ProcessIdentifier::VFSD, MessageType::Ipc) {
        Ok(parts) => {
            for part in parts {
                response_context.send(&part);
            }
        },
        Err(e) => {
            ::syslog::error!("finish_getdents: into_parts failed (error={:?})", e);
            response_context.send(&build_error(source_tid, ErrorCode::IoErr));
        },
    }
}

/// Drives a getdents sweep forward after an entry (or end-of-directory) was processed.
///
/// On [`GetdentsStep::Continue`], issues the next readdir request reusing `op_id` and
/// leaves the pending op buffered for the next response. On [`GetdentsStep::Done`],
/// removes the op and sends the assembled directory listing. If issuing the next
/// request fails, the pending op is cancelled so the caller is not left blocked.
pub(crate) fn drive_getdents(queue: &mut PendingQueue, op_id: OperationId, step: GetdentsStep) {
    match step {
        GetdentsStep::Continue { remote_fd, offset } => {
            if let Err(e) = crate::hostfs::send_readdir_request(remote_fd, offset, op_id) {
                ::syslog::error!(
                    "drive_getdents: send_readdir_request failed (op_id={}, remote_fd={}, \
                     offset={}, error={:?})",
                    op_id,
                    remote_fd,
                    offset,
                    e
                );
                if let Some(op) = queue.remove(op_id) {
                    cancel_pending_op(op, ErrorCode::IoErr);
                }
            }
        },
        GetdentsStep::Done => {
            if let Some(op) = queue.remove(op_id) {
                finish_getdents(op);
            }
        },
        GetdentsStep::Failed(code) => {
            if let Some(op) = queue.remove(op_id) {
                cancel_pending_op(op, code);
            }
        },
    }
}

/// Multi-part hostfs response stream handled by [`accumulate_response_part`].
#[derive(Clone, Copy)]
pub(crate) enum LongResponseStream {
    /// Long symbolic-link target returned by `readlink`.
    Readlink,
    /// Long directory entry name returned by `readdir`.
    ReadDir,
}

impl LongResponseStream {
    /// Returns the stream name used in log messages.
    fn label(self) -> &'static str {
        match self {
            Self::Readlink => "readlink",
            Self::ReadDir => "readdir",
        }
    }

    /// Returns the largest number of parts a well-formed stream may advertise.
    fn max_parts(self) -> usize {
        let body_size: usize = match self {
            Self::Readlink => {
                ::sysapi::limits::PATH_MAX + ::hostfs_api::long_msg::READLINK_RESPONSE_HEADER_SIZE
            },
            Self::ReadDir => {
                ::sysapi::limits::NAME_MAX + ::hostfs_api::long_msg::READDIR_RESPONSE_HEADER_SIZE
            },
        };
        body_size.div_ceil(::syscall::message::SystemCallMessagePart::PAYLOAD_SIZE)
    }
}

fn fail_response(queue: &mut PendingQueue, op_id: OperationId) {
    if let Some(op) = queue.remove(op_id) {
        cancel_pending_op(op, ErrorCode::IoErr);
    } else {
        queue.discard_abandoned_operation(op_id);
    }
}

/// Accumulates one part of a multi-part hostfs *response* stream into `slot`.
///
/// Shared by the long-target `readlink` and long-name `readdir` response paths, which
/// use identical framing: part 0 carries the op_id in its first 4 bytes, and the
/// assembled body is the concatenation of every part's payload. `stream` selects the
/// advertised part-count bound and names the stream in log messages.
///
/// `slot` holds the single in-flight assembler (hostfsd's single-threaded worker
/// guarantees at most one stream is in flight at a time). On a fresh
/// `part_number == 0`, any incomplete stream already in `slot` is discarded and its
/// pending op cancelled. Allocation or `add_part` failures also cancel the pending op
/// and clear `slot`.
///
/// Returns `Some((body, op_id))` once the stream is complete and ready for dispatch,
/// or `None` when more parts are required or the part was dropped (all error handling,
/// including pending-op cancellation, is performed internally).
pub(crate) fn accumulate_response_part(
    slot: &mut Option<(::syscall::message::SystemCallLongMessage, OperationId)>,
    queue: &mut PendingQueue,
    part: ::syscall::message::SystemCallMessagePart,
    outer_op_id: OperationId,
    stream: LongResponseStream,
) -> Option<(::alloc::vec::Vec<u8>, OperationId)> {
    let label: &str = stream.label();
    let total_parts: usize = part.total_parts as usize;
    let max_parts: usize = stream.max_parts();
    if total_parts == 0 || total_parts > max_parts {
        ::syslog::error!(
            "{} response advertises invalid part count (total_parts={}, max_parts={})",
            label,
            total_parts,
            max_parts
        );
        fail_response(queue, outer_op_id);
        return None;
    }

    // A fresh stream starts at part 0: validate, extract the op_id, drop any stale
    // stream, and allocate the assembler.
    if part.part_number == 0 {
        if (part.payload_size as usize) < OperationId::SERIALIZED_SIZE {
            ::syslog::error!(
                "{} response part 0 too short to carry op_id (payload_size={})",
                label,
                part.payload_size
            );
            fail_response(queue, outer_op_id);
            return None;
        }
        let op_id: OperationId = OperationId::from_le_bytes([
            part.payload[0],
            part.payload[1],
            part.payload[2],
            part.payload[3],
        ]);
        if op_id != outer_op_id {
            ::syslog::error!(
                "{} response identifier mismatch (outer_op_id={}, body_op_id={})",
                label,
                outer_op_id,
                op_id
            );
            fail_response(queue, outer_op_id);
            return None;
        }
        if let Some((_, stale_op_id)) = slot.take() {
            ::syslog::warn!(
                "discarding incomplete {} response stream on new part-0 arrival (cancelling stale \
                 op_id={})",
                label,
                stale_op_id
            );
            fail_response(queue, stale_op_id);
        }
        let capacity: usize = total_parts;
        match ::syscall::message::SystemCallLongMessage::new(capacity) {
            Ok(asm) => {
                *slot = Some((asm, op_id));
            },
            Err(e) => {
                // Allocation failure: cancel the caller now rather than letting the
                // pending op linger until eviction.
                ::syslog::error!(
                    "failed to allocate {} response assembler (op_id={}, capacity={}, error={:?})",
                    label,
                    op_id,
                    capacity,
                    e
                );
                *slot = None;
                fail_response(queue, op_id);
                return None;
            },
        }
    }

    if slot
        .as_ref()
        .is_some_and(|(_, op_id)| *op_id != outer_op_id)
    {
        let (_, active_op_id) = slot.take().unwrap();
        ::syslog::error!(
            "{} response stream identifier changed (active_op_id={}, outer_op_id={})",
            label,
            active_op_id,
            outer_op_id
        );
        fail_response(queue, active_op_id);
        fail_response(queue, outer_op_id);
        return None;
    }

    if let Some((asm, op_id)) = slot.as_mut() {
        let op_id_copy: OperationId = *op_id;
        if let Err(e) = asm.add_part(part) {
            ::syslog::error!(
                "failed to add {} response part (op_id={}, error={:?})",
                label,
                op_id_copy,
                e
            );
            *slot = None;
            fail_response(queue, op_id_copy);
            return None;
        }
        if asm.is_complete() {
            let (asm_done, _) = slot.take().unwrap();
            let mut body: ::alloc::vec::Vec<u8> = ::alloc::vec::Vec::new();
            for p in asm_done.take_parts() {
                let n: usize = p.payload_size as usize;
                body.extend_from_slice(&p.payload[..n]);
            }
            return Some((body, op_id_copy));
        }
        None
    } else {
        // Copy the field out of the packed `SystemCallMessagePart` before logging:
        // taking a reference to a misaligned packed field is undefined behavior.
        let pn: u16 = part.part_number;
        ::syslog::warn!(
            "{} response part received without active assembler (part_number={})",
            label,
            pn
        );
        fail_response(queue, outer_op_id);
        None
    }
}

/// Checks that the response payload header matches the expected operation kind.
///
/// Returns `true` if the header is valid for this operation, `false` if desync is detected.
fn validate_response_header(kind: &PendingOpKind, payload: &[u8; Message::PAYLOAD_SIZE]) -> bool {
    use ::syscall::SystemCallMessageHeader;

    let header_raw: u16 = u16::from_ne_bytes([payload[0], payload[1]]);
    let header: SystemCallMessageHeader = match SystemCallMessageHeader::try_from(header_raw) {
        Ok(h) => h,
        Err(_) => return false,
    };

    matches!(
        (kind, header),
        (PendingOpKind::Open { .. }, SystemCallMessageHeader::HostFsOpenResponse)
            | (PendingOpKind::Close, SystemCallMessageHeader::HostFsCloseResponse)
            | (PendingOpKind::Read { .. }, SystemCallMessageHeader::HostFsReadResponse)
            | (PendingOpKind::Write { .. }, SystemCallMessageHeader::HostFsWriteResponse)
            | (PendingOpKind::Seek { .. }, SystemCallMessageHeader::HostFsLseekResponse)
            | (PendingOpKind::Flush, SystemCallMessageHeader::HostFsFlushResponse)
            | (PendingOpKind::Truncate, SystemCallMessageHeader::HostFsTruncateResponse)
            | (PendingOpKind::Mkdir, SystemCallMessageHeader::HostFsMkdirResponse)
            | (PendingOpKind::Rmdir, SystemCallMessageHeader::HostFsRmdirResponse)
            | (PendingOpKind::Unlink, SystemCallMessageHeader::HostFsUnlinkResponse)
            | (PendingOpKind::Rename, SystemCallMessageHeader::HostFsRenameResponse)
            | (PendingOpKind::Stat, SystemCallMessageHeader::HostFsStatResponse)
            | (PendingOpKind::Symlink, SystemCallMessageHeader::HostFsSymlinkResponse)
            | (PendingOpKind::Readlink { .. }, SystemCallMessageHeader::HostFsReadlinkResponse)
            | (PendingOpKind::Lstat, SystemCallMessageHeader::HostFsLstatResponse)
            | (PendingOpKind::PathStat, SystemCallMessageHeader::HostFsPathStatResponse)
            | (PendingOpKind::Getdents { .. }, SystemCallMessageHeader::HostFsReadDirResponse)
    )
}

//==================================================================================================
// Completion Helpers
//==================================================================================================

fn complete_open(
    response_context: ResponseContext,
    response_payload: &[u8; Message::PAYLOAD_SIZE],
    path: alloc::string::String,
) {
    use ::syscall::fcntl::message::OpenAtResponse;

    let source_tid: ThreadIdentifier = response_context.source_tid();
    let resp: ::hostfs_api::OpenResponse = ::hostfs_api::OpenResponse::decode(response_payload);
    if resp.fd < 0 {
        let code: ErrorCode = hostfs_error_to_code(resp.fd);
        response_context.send(&build_error(source_tid, code));
        return;
    }
    let is_dir: bool = resp.is_dir != 0;
    match ::vfs::fd::vfs_alloc_hostfs(resp.fd, is_dir, if is_dir { Some(path) } else { None }) {
        Ok(local_fd) => {
            let epoch: u64 = ::vfs::fd::vfs_current_generation();
            let msg: Message = OpenAtResponse::build(
                source_tid,
                local_fd,
                epoch,
                ProcessIdentifier::VFSD,
                MessageType::Ipc,
            );
            response_context.send(&msg);
        },
        Err(_) => {
            // Issue a best-effort close to hostfsd so the remote FD does not leak.
            // We tag the request with the `FIRE_AND_FORGET` sentinel op_id and do not register
            // a pending op; the main loop recognizes that sentinel on hostfsd's response and
            // discards it without logging, since no pending entry exists.
            let _ = crate::hostfs::send_close_request(
                resp.fd,
                ::hostfs_api::OperationId::FIRE_AND_FORGET,
            );
            response_context.send(&build_error(source_tid, ErrorCode::TooManyOpenFiles));
        },
    }
}

fn complete_close(
    response_context: ResponseContext,
    response_payload: &[u8; Message::PAYLOAD_SIZE],
) {
    use ::syscall::unistd::message::CloseResponse;

    let source_tid: ThreadIdentifier = response_context.source_tid();
    // Check if hostfsd reported an error (status in the data portion).
    let ds: usize = ::hostfs_api::HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response_payload[ds..ds + 4].try_into().unwrap_or([0; 4]));
    if status < 0 {
        response_context.send(&build_error(source_tid, hostfs_error_to_code(status)));
        return;
    }
    let msg: Message =
        CloseResponse::build(source_tid, 0, ProcessIdentifier::VFSD, MessageType::Ipc);
    response_context.send(&msg);
}

fn complete_read(pending: &PendingOp) -> bool {
    use ::syscall::unistd::message::ReadResponse as SyscallReadResponse;

    let PendingOpKind::Read {
        count,
        fd,
        remote_fd,
        offset,
        response,
        delivery_error,
        ..
    } = &pending.kind
    else {
        ::syslog::error!("HostFS read delivery is missing its response");
        return true;
    };
    let response_context: ResponseContext = pending.response_context;
    let source_pid: ProcessIdentifier = response_context.source_pid();
    let source_tid: ThreadIdentifier = response_context.source_tid();
    if let Some(code) = *delivery_error {
        return match fail_pending_read(pending, code) {
            Ok(()) => true,
            Err(error)
                if matches!(error.code, ErrorCode::NoSuchEntry | ErrorCode::NoSuchProcess) =>
            {
                true
            },
            Err(error) => {
                ::syslog::warn!(
                    "HostFS read error delivery is not ready (pid={:?}, tid={:?}, error={:?})",
                    source_pid,
                    source_tid,
                    error
                );
                false
            },
        };
    }
    let Some(resp) = response else {
        ::syslog::error!("HostFS read delivery is missing its response");
        return true;
    };
    if resp.bytes_read < 0 {
        match ::sys::kcall::ipc::__kcall_push_tagged_timed(
            source_pid,
            source_tid,
            &[],
            response_context.request_id(),
            Some(Duration::ZERO),
        ) {
            Ok(()) => {},
            Err(error) if error.code == ErrorCode::OperationTimedOut => return false,
            Err(error) => {
                ::syslog::warn!("HostFS read error delivery failed (error={:?})", error);
            },
        }
        response_context.send(&build_error(source_tid, hostfs_error_to_code(resp.bytes_read)));
        return true;
    }
    let n: usize = (resp.bytes_read as usize).min(*count);
    if let Err(e) = ::sys::kcall::ipc::__kcall_push_tagged_timed(
        source_pid,
        source_tid,
        &resp.data[..n],
        response_context.request_id(),
        Some(Duration::ZERO),
    ) {
        if e.code == ErrorCode::OperationTimedOut {
            return false;
        }
        ::syslog::error!("hostfs read complete: push failed (error={:?})", e);
        response_context.send(&build_error(source_tid, ErrorCode::IoErr));
        return true;
    }
    update_hostfs_offset(*fd, *remote_fd, *offset, offset.saturating_add(n as i64));
    let msg: Message = SyscallReadResponse::build(
        source_tid,
        n as i32,
        [0u8; SyscallReadResponse::BUFFER_SIZE],
        ProcessIdentifier::VFSD,
        MessageType::Ipc,
    );
    response_context.send(&msg);
    true
}

fn schedule_read_retry(op_id: OperationId, delay: Duration) -> Result<(), ::sys::error::Error> {
    let mut now: SystemTime = SystemTime::default();
    ::sys::kcall::pm::__kcall_gettime(&mut now)?;
    let deadline: SystemTime = now.checked_add_duration(&delay).ok_or_else(|| {
        ::sys::error::Error::new(ErrorCode::ValueOutOfRange, "HostFS retry deadline overflow")
    })?;

    lock_read_retry_schedule()?;
    READ_RETRY_SCHEDULE.lock().insert(op_id, deadline);
    let signal_result: Result<(), ::sys::error::Error> =
        ::syscall::pthread::pthread_cond_signal(&READ_RETRY_CONDITION);
    if signal_result.is_err() {
        READ_RETRY_SCHEDULE.lock().remove(&op_id);
    }
    let unlock_result: Result<(), ::sys::error::Error> = unlock_read_retry_schedule();
    signal_result?;
    unlock_result
}

fn cancel_scheduled_read_retry(op_id: OperationId) {
    READ_RETRY_SCHEDULE.lock().remove(&op_id);
}

fn lock_read_retry_schedule() -> Result<(), ::sys::error::Error> {
    // SAFETY: every reference to this static mutex is confined to the pthread synchronization
    // API; no caller accesses its storage directly.
    unsafe {
        ::syscall::pthread::pthread_mutex_lock(&mut *core::ptr::addr_of_mut!(READ_RETRY_MUTEX))
    }
}

fn unlock_read_retry_schedule() -> Result<(), ::sys::error::Error> {
    // SAFETY: this is paired with `lock_read_retry_schedule()` on the calling thread.
    unsafe {
        ::syscall::pthread::pthread_mutex_unlock(&mut *core::ptr::addr_of_mut!(READ_RETRY_MUTEX))
    }
}

/// Runs the timer thread that converts due HostFS read deadlines into main-loop messages.
pub(crate) extern "C" fn read_retry_scheduler(main_tid_raw: usize) -> usize {
    let main_tid: ThreadIdentifier = match ThreadIdentifier::try_from(main_tid_raw) {
        Ok(tid) => tid,
        Err(error) => {
            ::syslog::error!(
                "HostFS read retry scheduler has invalid main TID (error={:?})",
                error
            );
            return 1;
        },
    };

    loop {
        if let Err(error) = lock_read_retry_schedule() {
            ::syslog::error!("HostFS read retry scheduler lock failed (error={:?})", error);
            return 1;
        }

        let deadline: Option<SystemTime> = READ_RETRY_SCHEDULE.lock().values().min().copied();
        // SAFETY: the scheduler holds `READ_RETRY_MUTEX`; the condition wait atomically releases
        // it while blocked and reacquires it before returning.
        let wait_result: Result<(), ::sys::error::Error> = unsafe {
            ::syscall::pthread::pthread_cond_timedwait(
                &READ_RETRY_CONDITION,
                &*core::ptr::addr_of!(READ_RETRY_MUTEX),
                deadline,
            )
        };
        if let Err(error) = wait_result {
            if error.code != ErrorCode::OperationTimedOut {
                ::syslog::warn!("HostFS read retry scheduler wait failed (error={:?})", error);
            }
        }

        if READ_RETRY_SCHEDULE.lock().is_empty() {
            if let Err(error) = unlock_read_retry_schedule() {
                ::syslog::error!("HostFS read retry scheduler unlock failed (error={:?})", error);
                return 1;
            }
            continue;
        }

        let mut now: SystemTime = SystemTime::default();
        if let Err(error) = ::sys::kcall::pm::__kcall_gettime(&mut now) {
            ::syslog::error!("HostFS read retry scheduler clock failed (error={:?})", error);
            if let Err(unlock_error) = unlock_read_retry_schedule() {
                ::syslog::error!(
                    "HostFS read retry scheduler unlock failed (error={:?})",
                    unlock_error
                );
                return 1;
            }
            continue;
        }

        let due: Vec<OperationId> = {
            let mut schedule = READ_RETRY_SCHEDULE.lock();
            let due: Vec<OperationId> = schedule
                .iter()
                .filter_map(|(op_id, deadline)| (now >= *deadline).then_some(*op_id))
                .collect();
            for op_id in &due {
                schedule.remove(op_id);
            }
            due
        };

        if let Err(error) = unlock_read_retry_schedule() {
            ::syslog::error!("HostFS read retry scheduler unlock failed (error={:?})", error);
            return 1;
        }

        for op_id in due {
            if let Err(error) = send_read_retry(op_id, main_tid) {
                ::syslog::error!(
                    "HostFS read retry scheduler send failed (op_id={}, error={:?})",
                    op_id,
                    error
                );
                if let Err(schedule_error) =
                    schedule_read_retry(op_id, READ_DELIVERY_RETRY_BASE_DELAY)
                {
                    ::syslog::error!(
                        "HostFS read retry scheduler requeue failed (op_id={}, error={:?})",
                        op_id,
                        schedule_error
                    );
                }
            }
        }
    }
}

fn send_read_retry(
    op_id: OperationId,
    main_tid: ThreadIdentifier,
) -> Result<(), ::sys::error::Error> {
    let mut payload: [u8; ::syscall::SystemCallMessage::PAYLOAD_SIZE] =
        [0u8; ::syscall::SystemCallMessage::PAYLOAD_SIZE];
    payload[..OperationId::SERIALIZED_SIZE].copy_from_slice(&op_id.to_le_bytes());
    let request: ::syscall::SystemCallMessage = ::syscall::SystemCallMessage::new(
        ::syscall::SystemCallMessageHeader::HostFsReadRetry,
        payload,
    );
    let message: Message = Message::new(
        ::sys::ipc::MessageSender::VFSD,
        ::sys::ipc::MessageReceiver::new(ProcessIdentifier::VFSD, main_tid),
        MessageType::Ipc,
        None,
        request.into_bytes(),
    );
    ::sys::kcall::ipc::__kcall_send(&message)
}

fn complete_write(
    response_context: ResponseContext,
    fd: i32,
    remote_fd: i32,
    offset: i64,
    response_payload: &[u8; Message::PAYLOAD_SIZE],
) {
    use ::syscall::unistd::message::WriteResponse as SyscallWriteResponse;

    let source_tid: ThreadIdentifier = response_context.source_tid();
    let resp: ::hostfs_api::WriteResponse = ::hostfs_api::WriteResponse::decode(response_payload);
    if resp.bytes_written < 0 {
        response_context.send(&build_error(source_tid, hostfs_error_to_code(resp.bytes_written)));
        return;
    }
    let next_offset: i64 = if resp.offset >= 0 {
        resp.offset
    } else {
        offset.saturating_add(resp.bytes_written as i64)
    };
    update_hostfs_offset(fd, remote_fd, offset, next_offset);
    let msg: Message = SyscallWriteResponse::build(
        source_tid,
        resp.bytes_written,
        ProcessIdentifier::VFSD,
        MessageType::Ipc,
    );
    response_context.send(&msg);
}

fn complete_seek(
    response_context: ResponseContext,
    fd: i32,
    remote_fd: i32,
    previous_offset: i64,
    response_payload: &[u8; Message::PAYLOAD_SIZE],
) {
    use ::syscall::unistd::message::SeekResponse;

    let source_tid: ThreadIdentifier = response_context.source_tid();
    let resp: ::hostfs_api::LseekResponse = ::hostfs_api::LseekResponse::decode(response_payload);
    if resp.offset < 0 {
        response_context.send(&build_error(source_tid, hostfs_error_to_code(resp.offset as i32)));
        return;
    }
    update_hostfs_offset(fd, remote_fd, previous_offset, resp.offset);
    let msg: Message =
        SeekResponse::build(source_tid, resp.offset, ProcessIdentifier::VFSD, MessageType::Ipc);
    response_context.send(&msg);
}

fn update_hostfs_offset(fd: i32, remote_fd: i32, expected: i64, next: i64) {
    use ::sysapi::unistd::file_seek::{
        SEEK_CUR,
        SEEK_SET,
    };

    if ::vfs::fd::vfs_hostfs_remote_fd(fd) != Some(remote_fd)
        || ::vfs::fd::vfs_lseek(fd, 0, SEEK_CUR) != Ok(expected)
    {
        return;
    }
    if let Err(error) = ::vfs::fd::vfs_lseek(fd, next, SEEK_SET) {
        ::syslog::warn!("failed to update hostfs virtual offset (fd={}, error={:?})", fd, error);
    }
}

/// Groups of operations that share the same "decode status code, send success/error" pattern.
enum OpGroup {
    Flush,
    Truncate,
    Mkdir,
    Rmdir,
    Unlink,
    Rename,
    Symlink,
}

fn complete_status(
    response_context: ResponseContext,
    response_payload: &[u8; Message::PAYLOAD_SIZE],
    group: OpGroup,
) {
    let source_tid: ThreadIdentifier = response_context.source_tid();
    let ds: usize = ::hostfs_api::HOSTFS_DATA_START;
    let status: i32 = i32::from_le_bytes(response_payload[ds..ds + 4].try_into().unwrap_or([0; 4]));
    if status < 0 {
        response_context.send(&build_error(source_tid, hostfs_error_to_code(status)));
        return;
    }
    let msg: Message = match group {
        OpGroup::Flush => {
            use ::syscall::unistd::message::FileSyncResponse;
            FileSyncResponse::build(source_tid, 0, ProcessIdentifier::VFSD, MessageType::Ipc)
        },
        OpGroup::Truncate => {
            use ::syscall::unistd::message::FileTruncateResponse;
            FileTruncateResponse::build(source_tid, 0, ProcessIdentifier::VFSD, MessageType::Ipc)
        },
        OpGroup::Mkdir => {
            use ::syscall::sys::stat::message::MakeDirectoryAtResponse;
            MakeDirectoryAtResponse::build(source_tid, 0, ProcessIdentifier::VFSD, MessageType::Ipc)
        },
        OpGroup::Rmdir | OpGroup::Unlink => {
            use ::syscall::fcntl::message::UnlinkAtResponse;
            UnlinkAtResponse::build(source_tid, 0, ProcessIdentifier::VFSD, MessageType::Ipc)
        },
        OpGroup::Rename => {
            use ::syscall::fcntl::message::RenameAtResponse;
            RenameAtResponse::build(source_tid, 0, ProcessIdentifier::VFSD, MessageType::Ipc)
        },
        OpGroup::Symlink => {
            use ::syscall::unistd::message::SymbolicLinkAtResponse;
            SymbolicLinkAtResponse::build(source_tid, 0, ProcessIdentifier::VFSD, MessageType::Ipc)
        },
    };
    response_context.send(&msg);
}

/// Maps a negative hostfsd error code back to an [`ErrorCode`].
///
/// These codes are defined in the `hostfs-api` crate as `HOSTFS_ERR_*` constants.
fn hostfs_error_to_code(code: i32) -> ErrorCode {
    match code {
        ::hostfs_api::HOSTFS_ERR_NOT_FOUND => ErrorCode::NoSuchEntry,
        ::hostfs_api::HOSTFS_ERR_PERMISSION => ErrorCode::PermissionDenied,
        ::hostfs_api::HOSTFS_ERR_EXISTS => ErrorCode::EntryExists,
        ::hostfs_api::HOSTFS_ERR_NOT_DIR => ErrorCode::InvalidDirectory,
        ::hostfs_api::HOSTFS_ERR_IS_DIR => ErrorCode::IsDirectory,
        ::hostfs_api::HOSTFS_ERR_INVALID => ErrorCode::InvalidArgument,
        ::hostfs_api::HOSTFS_ERR_NOT_EMPTY => ErrorCode::DirectoryNotEmpty,
        ::hostfs_api::HOSTFS_ERR_LOOP => ErrorCode::SymbolicLinkLoop,
        ::hostfs_api::HOSTFS_ERR_NOT_SUPPORTED => ErrorCode::OperationNotSupported,
        _ => ErrorCode::IoErr,
    }
}

fn complete_stat(
    response_context: ResponseContext,
    response_payload: &[u8; Message::PAYLOAD_SIZE],
) {
    use ::sysapi::{
        sys_stat::{
            file_mode,
            file_type,
            stat,
        },
        sys_types::off_t,
        time::timespec,
    };
    use ::syscall::{
        message::MessagePartitioner,
        sys::stat::message::FileStatAtResponse,
    };

    let source_tid: ThreadIdentifier = response_context.source_tid();
    let resp: ::hostfs_api::StatResponse = ::hostfs_api::StatResponse::decode(response_payload);

    // Check the explicit status field for errors.
    if resp.status < 0 {
        let code: ErrorCode = hostfs_error_to_code(resp.status);
        response_context.send(&build_error(source_tid, code));
        return;
    }

    let is_dir: bool = resp.is_dir != 0;
    let mode: u32 = if resp.mode != 0 {
        // Use host-provided mode, adding file type bits.
        let type_bits: u32 = if is_dir {
            file_type::S_IFDIR
        } else {
            file_type::S_IFREG
        };
        type_bits | (resp.mode & 0o7777)
    } else {
        // Fallback: synthesize mode like local VFS does.
        if is_dir {
            file_type::S_IFDIR | file_mode::S_IRWXU
        } else {
            file_type::S_IFREG | file_mode::S_IRUSR | file_mode::S_IWUSR
        }
    };

    let st = stat {
        st_dev: STAT_HOSTFS_DEV,
        st_ino: STAT_SYNTHETIC_INO,
        st_mode: mode,
        st_nlink: if is_dir {
            STAT_NLINK_DIR
        } else {
            STAT_NLINK_FILE
        },
        st_uid: 0,
        st_gid: 0,
        st_rdev: 0,
        st_size: resp.size as off_t,
        st_atim: timespec {
            tv_sec: STAT_FIXED_EPOCH,
            tv_nsec: 0,
        },
        st_mtim: timespec {
            tv_sec: STAT_FIXED_EPOCH,
            tv_nsec: 0,
        },
        st_ctim: timespec {
            tv_sec: STAT_FIXED_EPOCH,
            tv_nsec: 0,
        },
        st_blksize: STAT_BLOCK_SIZE,
        st_blocks: resp.size.div_ceil(STAT_SECTOR_SIZE) as off_t,
    };

    let response: FileStatAtResponse = FileStatAtResponse::new(st);
    match response.into_parts(source_tid, ProcessIdentifier::VFSD, MessageType::Ipc) {
        Ok(parts) => {
            for part in parts {
                response_context.send(&part);
            }
        },
        Err(e) => {
            ::syslog::error!("complete_stat: into_parts failed (error={:?})", e);
            response_context.send(&build_error(source_tid, ErrorCode::IoErr));
        },
    }
}

/// Completes a long-form (multi-part) readlink response.
///
/// `body` is the assembled response body in the wire format
/// `[op_id:4][status:4][target_len:2][target:N]`. The op_id has already been
/// consumed by the caller to look up the pending op, but it remains in `body` and
/// is skipped here.
pub(crate) fn complete_readlink_long(pending: PendingOp, body: &[u8]) {
    use ::syscall::{
        message::MessagePartitioner,
        unistd::message::ReadLinkAtResponse,
    };

    let response_context: ResponseContext = pending.response_context;
    let source_tid: ThreadIdentifier = pending.source_tid;

    let PendingOpKind::Readlink { bufsiz } = pending.kind else {
        ::syslog::error!("complete_readlink_long: pending operation is not readlink");
        response_context.send(&build_error(source_tid, ErrorCode::IoErr));
        return;
    };

    let resp: ::hostfs_api::long_msg::LongReadlinkResponse<'_> =
        match ::hostfs_api::long_msg::deserialize_long_readlink_response(body) {
            Some(r) => r,
            None => {
                ::syslog::error!(
                    "complete_readlink_long: failed to deserialize response body (len={})",
                    body.len()
                );
                response_context.send(&build_error(source_tid, ErrorCode::IoErr));
                return;
            },
        };

    if resp.status < 0 {
        response_context.send(&build_error(source_tid, hostfs_error_to_code(resp.status)));
        return;
    }

    // Truncate to the caller's buffer size, matching POSIX readlink semantics.
    let copy_len: usize = resp.target.len().min(bufsiz);
    let buffer: alloc::vec::Vec<u8> = resp.target[..copy_len].to_vec();

    let response: ReadLinkAtResponse = match ReadLinkAtResponse::new(buffer) {
        Ok(r) => r,
        Err(e) => {
            ::syslog::error!("complete_readlink_long: build response failed (error={:?})", e);
            response_context.send(&build_error(source_tid, ErrorCode::IoErr));
            return;
        },
    };
    match response.into_parts(source_tid, ProcessIdentifier::VFSD, MessageType::Ipc) {
        Ok(parts) => {
            for part in parts {
                response_context.send(&part);
            }
        },
        Err(e) => {
            ::syslog::error!("complete_readlink_long: into_parts failed (error={:?})", e);
            response_context.send(&build_error(source_tid, ErrorCode::IoErr));
        },
    }
}

fn complete_readlink(
    response_context: ResponseContext,
    response_payload: &[u8; Message::PAYLOAD_SIZE],
    bufsiz: usize,
) {
    use ::syscall::{
        message::MessagePartitioner,
        unistd::message::ReadLinkAtResponse,
    };

    let source_tid: ThreadIdentifier = response_context.source_tid();
    let resp: ::hostfs_api::ReadlinkResponse =
        match ::hostfs_api::ReadlinkResponse::decode(response_payload) {
            Some(r) => r,
            None => {
                ::syslog::error!("complete_readlink: failed to decode response");
                response_context.send(&build_error(source_tid, ErrorCode::IoErr));
                return;
            },
        };

    if resp.status < 0 {
        response_context.send(&build_error(source_tid, hostfs_error_to_code(resp.status)));
        return;
    }

    let target_len: usize = resp.target_len as usize;
    let max: usize = target_len.min(resp.target.len()).min(bufsiz);
    let buffer: alloc::vec::Vec<u8> = resp.target[..max].to_vec();

    let response: ReadLinkAtResponse = match ReadLinkAtResponse::new(buffer) {
        Ok(r) => r,
        Err(e) => {
            ::syslog::error!("complete_readlink: build response failed (error={:?})", e);
            response_context.send(&build_error(source_tid, ErrorCode::IoErr));
            return;
        },
    };
    match response.into_parts(source_tid, ProcessIdentifier::VFSD, MessageType::Ipc) {
        Ok(parts) => {
            for part in parts {
                response_context.send(&part);
            }
        },
        Err(e) => {
            ::syslog::error!("complete_readlink: into_parts failed (error={:?})", e);
            response_context.send(&build_error(source_tid, ErrorCode::IoErr));
        },
    }
}

fn complete_lstat(
    response_context: ResponseContext,
    response_payload: &[u8; Message::PAYLOAD_SIZE],
) {
    use ::sysapi::{
        sys_stat::{
            file_mode,
            file_type,
            stat,
        },
        sys_types::off_t,
        time::timespec,
    };
    use ::syscall::{
        message::MessagePartitioner,
        sys::stat::message::FileStatAtResponse,
    };

    let source_tid: ThreadIdentifier = response_context.source_tid();
    let resp: ::hostfs_api::LstatResponse =
        match ::hostfs_api::LstatResponse::decode(response_payload) {
            Some(r) => r,
            None => {
                ::syslog::error!("complete_lstat: failed to decode response");
                response_context.send(&build_error(source_tid, ErrorCode::IoErr));
                return;
            },
        };

    if resp.status < 0 {
        response_context.send(&build_error(source_tid, hostfs_error_to_code(resp.status)));
        return;
    }

    let type_bits: u32 = match resp.kind {
        ::hostfs_api::file_kind::DIRECTORY => file_type::S_IFDIR,
        ::hostfs_api::file_kind::SYMLINK => file_type::S_IFLNK,
        ::hostfs_api::file_kind::REGULAR => file_type::S_IFREG,
        _ => file_type::S_IFREG,
    };
    // Mask `resp.mode` down to permission bits before OR-ing with our canonical
    // `type_bits`. On Unix hosts `resp.mode` is the raw `st_mode` (already includes
    // type bits); on Windows it is a synthetic value built by hostfsd that also
    // includes synthetic type bits. The mask strips those in both cases so the
    // file-type bits are unambiguously driven by `resp.kind` (the authoritative
    // discriminant on the wire).
    let mode: u32 = if resp.mode != 0 {
        type_bits | (resp.mode & 0o7777)
    } else {
        match resp.kind {
            ::hostfs_api::file_kind::DIRECTORY => file_type::S_IFDIR | file_mode::S_IRWXU,
            ::hostfs_api::file_kind::SYMLINK => {
                file_type::S_IFLNK | file_mode::S_IRUSR | file_mode::S_IWUSR
            },
            _ => file_type::S_IFREG | file_mode::S_IRUSR | file_mode::S_IWUSR,
        }
    };
    let is_dir: bool = resp.kind == ::hostfs_api::file_kind::DIRECTORY;

    let st = stat {
        st_dev: STAT_HOSTFS_DEV,
        st_ino: STAT_SYNTHETIC_INO,
        st_mode: mode,
        st_nlink: if is_dir {
            STAT_NLINK_DIR
        } else {
            STAT_NLINK_FILE
        },
        st_uid: 0,
        st_gid: 0,
        st_rdev: 0,
        st_size: resp.size as off_t,
        st_atim: timespec {
            tv_sec: STAT_FIXED_EPOCH,
            tv_nsec: 0,
        },
        st_mtim: timespec {
            tv_sec: STAT_FIXED_EPOCH,
            tv_nsec: 0,
        },
        st_ctim: timespec {
            tv_sec: STAT_FIXED_EPOCH,
            tv_nsec: 0,
        },
        st_blksize: STAT_BLOCK_SIZE,
        st_blocks: resp.size.div_ceil(STAT_SECTOR_SIZE) as off_t,
    };

    let response: FileStatAtResponse = FileStatAtResponse::new(st);
    match response.into_parts(source_tid, ProcessIdentifier::VFSD, MessageType::Ipc) {
        Ok(parts) => {
            for part in parts {
                response_context.send(&part);
            }
        },
        Err(e) => {
            ::syslog::error!("complete_lstat: into_parts failed (error={:?})", e);
            response_context.send(&build_error(source_tid, ErrorCode::IoErr));
        },
    }
}
