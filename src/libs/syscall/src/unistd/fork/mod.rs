// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::proc::{
    fork_sync_request,
    ForkSyncAckMessage,
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
        MessageType,
        SystemMessage,
        SystemMessageHeader,
    },
    kcall::fork::__kcall_fork,
    pm::ProcessIdentifier,
};
use ::sysapi::sys_types::pid_t;

//==================================================================================================
// Private Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Maps an error reported by the `duplicate()` kernel call onto the error code that `fork()` must
/// surface to user space.
///
/// # Parameters
///
/// - `code`: Error code reported by the kernel.
///
/// # Returns
///
/// The error code to surface to user space.
///
fn map_duplicate_error(code: ErrorCode) -> ErrorCode {
    match code {
        // Resource exhaustion is reported to user space as a transient failure (EAGAIN).
        ErrorCode::OutOfMemory => ErrorCode::TryAgain,
        // POSIX `fork()` does not define EPERM; a capability/permission failure to duplicate is
        // surfaced as the transient EAGAIN so that callers may reasonably retry.
        ErrorCode::OperationNotPermitted => ErrorCode::TryAgain,
        // Anything else is reported as insufficient memory (ENOMEM).
        _ => ErrorCode::OutOfMemory,
    }
}

///
/// # Description
///
/// Synchronizes a freshly forked parent with the process manager daemon.
///
/// After `fork()`, the kernel duplicates the parent's address space, but the child's filesystem
/// state (open file descriptors and current working directory) is duplicated asynchronously by the
/// filesystem daemon. To honor POSIX semantics, the parent must not mutate its descriptor table
/// before that snapshot is taken. This function blocks the parent until the process manager daemon
/// confirms that the fork-clone has been dispatched to the filesystem daemon, after which the
/// parent's subsequent filesystem operations are correctly ordered after the snapshot.
///
/// # Parameters
///
/// - `child`: Process identifier of the freshly forked child.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
fn sync_parent_after_fork(child: ProcessIdentifier) -> Result<(), Error> {
    // The request must carry the parent's own identity as its source so that the daemon knows which
    // process to release alongside the child.
    let parent: ProcessIdentifier = ::sys::kcall::pm::getpid()?;
    let request: Message = fork_sync_request(parent, child)?;
    if let Err(error) = ::sys::kcall::ipc::__kcall_send(&request) {
        ::syslog::error!("sync_parent_after_fork(): failed to send fork-sync request: {error:?}");
        return Err(error);
    }

    // Block until the daemon releases us.
    sync_child_after_fork()
}

///
/// # Description
///
/// Blocks until the process manager daemon acknowledges that the fork-clone of the calling process
/// has been dispatched to the filesystem daemon.
///
/// This is the child's half of the fork synchronization: a freshly forked child must not use its
/// inherited descriptors before the filesystem daemon has duplicated them. The parent triggers the
/// duplication (see [`sync_parent_after_fork`]); the daemon releases the parent and the child
/// together once the fork-clone has been dispatched.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
fn sync_child_after_fork() -> Result<(), Error> {
    // Block waiting for the acknowledgement. In the fork-synchronization window both the parent and
    // the child are blocked on the daemon, so the daemon is the only process messaging them: the
    // next message is the acknowledgement. This invariant holds only because no other party may
    // message a process inside this window — a child is not yet known to any peer, and the parent
    // is blocked here rather than servicing requests. A stray message would be reported as an error
    // (below) and fail `fork()`, rather than being silently mistaken for the acknowledgement.
    let message: Message = ::sys::kcall::ipc::__kcall_recv()?;

    match message.message_type {
        MessageType::Ipc => {
            let message: SystemMessage = SystemMessage::from_bytes(message.payload)?;
            match message.header {
                SystemMessageHeader::ProcessManagement => {
                    let message: ProcessManagementMessage =
                        ProcessManagementMessage::from_bytes(message.payload)?;
                    match message.header {
                        ProcessManagementMessageHeader::ForkSyncAck => {
                            // The acknowledgement carries the outcome of the fork synchronization. A
                            // non-zero status means the process manager daemon could not dispatch the
                            // fork-clone to the filesystem daemon, so `fork()` should fail rather than
                            // proceed past a snapshot that was never taken.
                            let ack: ForkSyncAckMessage =
                                ForkSyncAckMessage::from_bytes(message.payload);
                            let status: i32 = ack.status;
                            if status == ForkSyncAckMessage::STATUS_SUCCESS {
                                Ok(())
                            } else {
                                let reason: &str = "fork synchronization failed";
                                ::syslog::error!(
                                    "sync_child_after_fork(): {} (status={:?})",
                                    reason,
                                    status
                                );
                                let code: ErrorCode =
                                    ErrorCode::try_from(status).unwrap_or(ErrorCode::TryAgain);
                                Err(Error::new(code, reason))
                            }
                        },
                        header => {
                            let reason: &str = "unexpected process management message while \
                                                awaiting fork-sync ack";
                            ::syslog::error!(
                                "sync_child_after_fork(): {} (header={:?})",
                                reason,
                                header
                            );
                            Err(Error::new(ErrorCode::InvalidMessage, reason))
                        },
                    }
                },
                header => {
                    let reason: &str = "invalid system message type";
                    ::syslog::error!("sync_child_after_fork(): {} (header={:?})", reason, header);
                    Err(Error::new(ErrorCode::InvalidMessage, reason))
                },
            }
        },
        message_type => {
            let reason: &str = "invalid message type";
            ::syslog::error!(
                "sync_child_after_fork(): {} (message_type={:?})",
                reason,
                message_type
            );
            Err(Error::new(ErrorCode::InvalidMessage, reason))
        },
    }
}

//==================================================================================================
// Public Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Creates a new process by duplicating the calling process, following POSIX `fork()` semantics.
///
/// This is a thin wrapper over the [`__kcall_fork`] kernel-call wrapper, which performs the actual
/// context capture and process duplication. This function adapts the kernel-call result to the
/// POSIX contract: the child's process identifier is converted to `pid_t`, and kernel errors are
/// mapped onto the error codes that `fork()` must surface to user space.
///
/// After the address space has been duplicated, the parent and child synchronize with the process
/// manager daemon before returning. The kernel duplicates memory eagerly, but the child's
/// filesystem state (open file descriptors and current working directory) is duplicated by the
/// filesystem daemon out of band. The synchronization guarantees that this duplication has been
/// dispatched before either process resumes, so that the parent cannot mutate its descriptor table
/// before the snapshot is taken and the child observes valid, offset-sharing descriptors the
/// instant `fork()` returns — as POSIX requires.
///
/// # Returns
///
/// On success, the child's process identifier is returned in the parent and `0` is returned in the
/// child. On failure, an error code is returned (only in the parent).
///
// `#[inline(never)]` keeps `do_fork` a distinct stack frame so that the register/stack state
// captured by `__kcall_fork()` is well defined and identical for the parent and the freshly
// duplicated child, which both resume at the kernel-call return site within this frame.
#[inline(never)]
pub fn do_fork() -> Result<pid_t, ErrorCode> {
    let child: ProcessIdentifier = match __kcall_fork() {
        Ok(child) => child,
        Err(e) => return Err(map_duplicate_error(e.code)),
    };

    if child == ProcessIdentifier::from(0) {
        // Child: block until the daemon confirms that the inherited descriptors have been
        // duplicated into the filesystem daemon.
        if let Err(error) = sync_child_after_fork() {
            // The fork synchronization failed: either the process manager daemon could not dispatch
            // the fork-clone to the filesystem daemon (and released the child with a failure
            // acknowledgement), or an unexpected message was observed in the synchronization window.
            // Either way the child's inherited descriptors are not in a well-defined state, and
            // POSIX forbids `fork()` from returning -1 in the child. Self-terminate rather than
            // surface the failure to the application, honoring the contract that no child survives a
            // failed `fork()`. The parent independently fails `fork()` and best-effort terminates
            // this child, so self-exiting here merely makes the teardown prompt and race-free.
            //
            // NOTE: this self-termination must stay here in the child branch and not move into
            // `sync_child_after_fork()`, which the parent also calls to block for its own
            // acknowledgement: the parent must surface the failure, not self-terminate.
            ::syslog::error!(
                "do_fork(): child fork synchronization failed, self-terminating (error={error:?})"
            );
            // Non-zero status marks abnormal termination of the child.
            const FORK_FAILURE_EXIT_STATUS: i32 = 1;
            let Err(exit_error) = ::sys::kcall::pm::__kcall_exit(FORK_FAILURE_EXIT_STATUS);
            // `__kcall_exit()` does not return on success; reaching here means the exit kcall itself
            // failed. There is nothing safe for a half-synchronized child to do, so panic rather
            // than return a forbidden -1 from `fork()`; the panic handler tears the child down and
            // the parent's best-effort terminate is a further backstop.
            panic!("do_fork(): child exit kcall returned (error={exit_error:?})");
        }

        // Defensively refresh the child's cached process identifier.
        //
        // The child inherits `CACHED_PID` (holding the *parent's* pid) through copy-on-write
        // memory. `__kcall_fork()` already clears this crate instance's copy at the start of the
        // child path, and the kernel always reports the child's own identity — there is no window
        // in which `getpid()` returns the parent's pid — so the next cached query re-resolves to
        // the child regardless. This post-handshake clear is a low-cost safety net for the
        // libc-side identity cache; it adds at most one kernel transition per fork.
        //
        // It is NOT the capability fix. `CACHED_PID` is a per-link-unit static, so a process image
        // that links several independently-compiled copies of this crate (e.g. a C program linking
        // `libposix.a` and `libnvx_crt0.a`) carries multiple instances that no single invalidation
        // can reach. The capability-sensitive callers — `mmap`/`mprotect`/`munmap` in
        // `MemorySegment` and the heap allocator — therefore resolve the owning pid with
        // `getpid_uncached()` and never trust this cache for the kernel's `pid != caller_pid` check.
        //
        // Tracked by https://github.com/nanvix/nanvix/issues/2622: once a single unified pid cache
        // is guaranteed per image, this defensive re-invalidation can be dropped.
        ::sys::kcall::pm::invalidate_cached_pid();

        Ok(0)
    } else {
        // Parent: ask the daemon to duplicate our descriptors onto the child, and block until it
        // confirms before resuming.
        if let Err(error) = sync_parent_after_fork(child) {
            // The child was already created by `__kcall_fork()`. If the daemon released the
            // synchronization with a failure acknowledgement, the child received the same failure
            // and is self-terminating; if instead the synchronization failed before the daemon was
            // involved (e.g. the request could not be sent), the child is still blocked in
            // `sync_child_after_fork()` awaiting an acknowledgement that will never arrive. Either
            // way, best-effort terminate it so no child survives a failed `fork()`, honoring POSIX;
            // a redundant terminate of an already-exiting child is harmless and merely logged.
            if let Err(terminate_error) = ::sys::kcall::pm::__kcall_terminate(child) {
                ::syslog::error!(
                    "do_fork(): failed to terminate child {child:?} after fork-sync failure: \
                     {terminate_error:?}"
                );
            }
            return Err(error.code);
        }
        Ok(i32::from(child))
    }
}
