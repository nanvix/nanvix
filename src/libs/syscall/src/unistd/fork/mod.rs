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
/// confirms that the filesystem daemon has taken the fork-clone snapshot, after which the parent's
/// subsequent filesystem operations are correctly ordered after that snapshot.
///
/// # Parameters
///
/// - `child`: Process identifier of the freshly forked child.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
fn sync_parent_after_fork(
    child: ProcessIdentifier,
    token: &::sys::ipc::RequestToken,
    previous_signal_mask: &::sys::pm::SigSet,
) -> Result<(), Error> {
    // The request must carry the parent's own identity as its source so that the daemon knows which
    // process to release alongside the child. Every failure before the mask is restored must still
    // restore it, otherwise the parent resumes with every signal blocked forever.
    let send_result: Result<(), Error> = ::sys::kcall::pm::getpid()
        .and_then(|parent: ProcessIdentifier| fork_sync_request(parent, child))
        .and_then(|mut request: Message| crate::rpc::send_request_with_token(token, &mut request));
    let restore_result: Result<(), Error> = restore_signal_mask(previous_signal_mask);
    if let Err(error) = send_result {
        ::syslog::error!("sync_parent_after_fork(): failed to send fork-sync request: {error:?}");
        return Err(error);
    }
    restore_result?;

    // Block until the daemon releases us.
    sync_after_fork(token)
}

///
/// # Description
///
/// Blocks until the process manager daemon acknowledges that the filesystem daemon has taken the
/// fork-clone snapshot for the calling process.
///
/// This is the child's half of the fork synchronization: a freshly forked child must not use its
/// inherited descriptors before the filesystem daemon has duplicated them. The parent triggers the
/// duplication (see [`sync_parent_after_fork`]); the daemon releases the parent and the child
/// together once the filesystem daemon acknowledges that the snapshot has been taken.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
fn sync_after_fork(token: &::sys::ipc::RequestToken) -> Result<(), Error> {
    // Parent and child wait on copies of the token allocated before duplication. The process daemon
    // echoes that identifier in both acknowledgements, so unrelated mailbox traffic is filtered
    // rather than mistaken for the fork result.
    let message: Message = crate::rpc::recv_response(token)?;

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
                            // non-zero status means the fork-clone snapshot failed, so `fork()`
                            // should fail rather than proceed past a snapshot that was never taken.
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

/// Rebinds the inherited request token to the child thread and waits for its acknowledgement.
fn sync_child_after_fork(token: ::sys::ipc::RequestToken) -> Result<(), Error> {
    let tid: ::sys::pm::ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;
    // SAFETY: `do_fork()` blocks signal delivery before duplication and calls this function only
    // in the freshly forked, single-threaded child. The original mask is restored after rebinding.
    let token: ::sys::ipc::RequestToken = unsafe { token.rebind_after_fork(tid)? };
    sync_after_fork(&token)
}

/// Blocks every catchable signal and returns the calling thread's previous signal mask.
fn block_signals_for_fork() -> Result<::sys::pm::SigSet, Error> {
    let blocked: ::sys::pm::SigSet = !0;
    let mut previous: ::sys::pm::SigSet = 0;
    unsafe {
        ::sys::kcall::pm::__kcall_sigprocmask(::sys::pm::SIG_SETMASK, &blocked, &mut previous)?;
    }
    Ok(previous)
}

/// Restores the signal mask saved by [`block_signals_for_fork`].
fn restore_signal_mask(previous: &::sys::pm::SigSet) -> Result<(), Error> {
    unsafe {
        ::sys::kcall::pm::__kcall_sigprocmask(
            ::sys::pm::SIG_SETMASK,
            previous,
            core::ptr::null_mut(),
        )
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
/// acknowledged before either process resumes, so that the parent cannot mutate its descriptor table
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
    // Prevent a signal handler from entering the request matcher between duplication and the
    // child's request-state rebind. The original mask is restored in both processes immediately
    // after their request state is ready.
    let previous_signal_mask: ::sys::pm::SigSet =
        block_signals_for_fork().map_err(|error| error.code)?;
    let tid: ::sys::pm::ThreadIdentifier = match ::sys::kcall::pm::__kcall_gettid() {
        Ok(tid) => tid,
        Err(error) => {
            let _result: Result<(), Error> = restore_signal_mask(&previous_signal_mask);
            return Err(error.code);
        },
    };
    // A signal handler may call fork while the interrupted thread is awaiting an RPC response.
    // The child cannot inherit that request: its reply is addressed to the parent's pid/tid, and
    // child rebinding intentionally discards all inherited state except the fork barrier token.
    if ::sys::ipc::has_active_requests(tid) {
        let _result: Result<(), Error> = restore_signal_mask(&previous_signal_mask);
        return Err(ErrorCode::TryAgain);
    }
    // Reserve the synchronization identifier before duplication so the child inherits the same
    // identifier that the parent later sends to the process daemon.
    let token: ::sys::ipc::RequestToken = match crate::rpc::begin_request(ProcessIdentifier::PROCD)
    {
        Ok(token) => token,
        Err(_) => {
            let _result: Result<(), Error> = restore_signal_mask(&previous_signal_mask);
            return Err(ErrorCode::TryAgain);
        },
    };
    // Quiesce request and heap metadata while the kernel copies the address space. The guards are
    // copied with it and unlock each process's private mutex before either side resumes work.
    let request_state_guard: ::sys::ipc::RequestStateForkGuard =
        unsafe { ::sys::ipc::RequestStateForkGuard::acquire() };
    let heap_guard: ::sysalloc::HeapForkGuard = unsafe { ::sysalloc::HeapForkGuard::acquire() };
    let fork_result: Result<ProcessIdentifier, Error> = __kcall_fork();
    drop(heap_guard);
    drop(request_state_guard);
    let child: ProcessIdentifier = match fork_result {
        Ok(child) => child,
        Err(e) => {
            let _result: Result<(), Error> = restore_signal_mask(&previous_signal_mask);
            return Err(map_duplicate_error(e.code));
        },
    };

    if child == ProcessIdentifier::from(0) {
        // Child: block until the daemon confirms that the inherited descriptors have been
        // duplicated into the filesystem daemon.
        let synchronization: Result<(), Error> = sync_child_after_fork(token).and_then(|()| {
            restore_signal_mask(&previous_signal_mask)?;
            Ok(())
        });
        if let Err(error) = synchronization {
            // The fork synchronization failed: either the fork-clone snapshot failed (and the
            // process manager daemon released the child with a failure acknowledgement), or an
            // unexpected message was observed in the synchronization window.
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

        // No cache refresh is needed here: the child inherited the parent's `CACHED_PID` through
        // copy-on-write memory, but `__kcall_fork()` already invalidated it on the child path — at
        // the single, lowest-level choke point common to every fork caller — before returning here.
        // `CACHED_PID` is one unified per-image instance, so that single invalidation reaches every
        // reader (including the capability-sensitive `mmap`/`mprotect`/`munmap` callers), and the
        // next `getpid()` re-resolves to the child's own identity.
        Ok(0)
    } else {
        // Parent: ask the daemon to duplicate our descriptors onto the child, and block until it
        // confirms before resuming. The request is sent while signals remain blocked, then the
        // original mask is restored before waiting for the acknowledgement.
        if let Err(error) = sync_parent_after_fork(child, &token, &previous_signal_mask) {
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
