// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    assembler::{
        assemble_and_dispatch,
        purge_process,
        AssemblerEntry,
        AssemblerKey,
    },
    console_wait::ConsoleWaitTable,
    error::{
        build_error,
        ResponseContext,
    },
    handler,
    hostfs,
    networkd,
    pending::PendingQueue,
    pipe_wait::PipeWaitTable,
};
use ::proc::{
    exec_ack,
    fork_clone_ack,
    ExecMessage,
    ForkCloneAckMessage,
    ForkCloneMessage,
    ProcessExitMessage,
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
    ShutdownMessage,
    TerminalDetachMessage,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
        MessageSender,
        RequestIdentifier,
        SystemMessage,
        SystemMessageHeader,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::syscall::{
    message::SystemCallMessagePart,
    poll::input_message::{
        ConsoleReadCancel,
        PipeOpCancelRequest,
        PipeOpCancelResponse,
        PipeReadRetry,
    },
    SystemCallMessage,
    SystemCallMessageKind,
};
use alloc::{
    collections::BTreeMap,
    vec::Vec,
};

//==================================================================================================
// Helpers: Extract caller identity from message source
//==================================================================================================

/// Extracts the caller's process identifier from an IPC message.
///
/// This returns `message.source.pid`, the kernel-attested caller identity that the `send` kernel
/// call stamps on every message (see `src/kernel/src/ipc/send.rs`). Because the kernel — not the
/// sender — fills this field, it is authoritative and unforgeable, and it is correct by construction
/// in the cases where the historical `TID == PID` reconstruction failed: a process reached via
/// `fork()` + `execv()` keeps its PID but is assigned a new main-thread TID (`TID != PID`), and a
/// request issued by a non-main thread of a multi-threaded caller likewise has `TID != PID`. Keying
/// per-process VFS state (fd table and cwd) by this PID therefore no longer misattributes a child's
/// writes after it exits, nor corrupts I/O on inherited descriptors (nanvix/nanvix#2650, #2637,
/// #2529).
fn caller_pid(message: &Message) -> ProcessIdentifier {
    // `source` is a `Copy` field of a `repr(packed)` `Message`; copy it out before projecting into
    // it so that no reference to an unaligned field is formed.
    let source: MessageSender = message.source;
    source.pid
}

//==================================================================================================
// SystemMessage Handler (procd shutdown)
//==================================================================================================

fn handle_system_message(
    message: Message,
    assemblers: &mut BTreeMap<AssemblerKey, AssemblerEntry>,
    pending: &mut PendingQueue,
    console_wait: &mut ConsoleWaitTable,
    pipe_wait: &mut PipeWaitTable,
) -> Result<bool, Error> {
    // State-mutating process-management messages are privileged: only procd may direct them. The
    // caller (handle_ipc_message) routes here only when the kernel-attested `message.source.pid` is
    // PROCD, so a sender cannot reach this path by forging `message.source`. The debug assert below
    // documents that routing invariant and catches accidental direct calls in development builds.
    debug_assert_eq!(
        caller_pid(&message),
        ProcessIdentifier::PROCD,
        "handle_system_message invoked with non-procd client"
    );
    let response_context: ResponseContext =
        ResponseContext::new(message.source, RequestIdentifier::read_from(&message));
    let sys_msg: SystemMessage = SystemMessage::from_bytes(message.payload)?;
    match sys_msg.header {
        SystemMessageHeader::ProcessManagement => {
            let pm_msg: ProcessManagementMessage =
                ProcessManagementMessage::from_bytes(sys_msg.payload)?;
            match pm_msg.header {
                ProcessManagementMessageHeader::Shutdown => {
                    let shutdown: ShutdownMessage = ShutdownMessage::from_bytes(pm_msg.payload);
                    ::syslog::info!("shutting down (code={:?})...", shutdown.code);
                    Ok(true)
                },
                ProcessManagementMessageHeader::ForkClone => {
                    let fork_clone: ForkCloneMessage = ForkCloneMessage::from_bytes(pm_msg.payload);
                    let parent: ProcessIdentifier = fork_clone.parent;
                    let child: ProcessIdentifier = fork_clone.child;
                    // If this is the root's first VFS-visible event, seed its console descriptors
                    // before cloning so the child inherits them. The helper is root-only, so this
                    // is a no-op for ordinary fork-clone notifications.
                    ::vfs::fd::vfs_seed_root_console(parent);
                    // Duplicate the parent's filesystem state onto the child: its open file
                    // descriptors (sharing the underlying open file descriptions, and therefore
                    // file offsets, as POSIX requires) together with a private copy of its current
                    // working directory.
                    let status: i32 = match ::vfs::fd::vfs_fork_clone(parent, child) {
                        Ok(()) => {
                            ::syslog::info!(
                                "cloned filesystem state (parent={:?}, child={:?})",
                                parent,
                                child
                            );
                            ForkCloneAckMessage::STATUS_SUCCESS
                        },
                        Err(e) => {
                            ::syslog::error!(
                                "failed to clone filesystem state (parent={:?}, child={:?}, \
                                 error={:?})",
                                parent,
                                child,
                                e
                            );
                            ErrorCode::TryAgain.get()
                        },
                    };
                    // Acknowledge the process manager daemon that the clone has been processed, so
                    // it releases the parent and child held at the fork-synchronization barrier only
                    // now that the snapshot has actually been taken. Releasing on dispatch alone
                    // would let the child run its first filesystem operation (e.g. the `execv()`
                    // image load) before this clone, so that operation would make the child's table
                    // active and the clone would be refused, dropping the inherited descriptors. A
                    // non-zero status reports a failed clone so the fork is aborted rather than
                    // proceeding with a half-cloned child.
                    match fork_clone_ack(
                        ProcessIdentifier::VFSD,
                        ProcessIdentifier::PROCD,
                        child,
                        status,
                    ) {
                        Ok(ack) => response_context.send(&ack),
                        Err(e) => ::syslog::error!(
                            "failed to build fork-clone acknowledgement (child={:?}, error={:?})",
                            child,
                            e
                        ),
                    }
                    Ok(false)
                },
                ProcessManagementMessageHeader::TerminalDetach => {
                    let detach: TerminalDetachMessage =
                        TerminalDetachMessage::from_bytes(pm_msg.payload);
                    ::vfs::fd::vfs_detach_controlling_terminal(detach.pid);
                    Ok(false)
                },
                ProcessManagementMessageHeader::ProcessExit => {
                    let exit: ProcessExitMessage = ProcessExitMessage::from_bytes(pm_msg.payload);
                    let pid: ProcessIdentifier = exit.pid;
                    // Reclaim the terminated process's per-process filesystem state, dropping its
                    // open file descriptors so that surviving siblings keep correct last-reference
                    // accounting. Any host-backed descriptors for which the process held the final
                    // reference can no longer be closed by the process itself, so close them on
                    // hostfsd here to avoid leaking remote handles. Pipe ends for which it held the
                    // final reference have their counts drop to zero, which must fire EOF/`EPIPE`
                    // wakeups for any suspended counterparts.
                    let reclaim: ::vfs::fd::ProcessExitReclaim = ::vfs::fd::vfs_process_exit(pid);
                    // First discard any requests this process had parked: there is no longer a
                    // client to answer, so they must not be revived by the wakeups below.
                    purge_process(assemblers, pid);
                    pending.purge_pid(pid);
                    console_wait.purge_pid(pid);
                    pipe_wait.purge_pid(pid);
                    for closure in reclaim.pipe_closures {
                        if closure.was_write {
                            handler::pipe::wake_all_readers_eof(closure.pipe_id, pipe_wait);
                        } else {
                            handler::pipe::fail_all_writers_epipe(closure.pipe_id, pipe_wait);
                        }
                    }
                    for remote_fd in reclaim.orphaned_hostfs_fds {
                        // Fire-and-forget close: the requesting process is gone, so there is no
                        // caller to acknowledge. As in the leak-avoidance path in `complete_open`,
                        // the request is tagged with the `FIRE_AND_FORGET` sentinel op_id and no
                        // pending op is registered; the main event loop recognizes that sentinel on
                        // hostfsd's response and discards it without logging.
                        if let Err(e) = hostfs::send_close_request(
                            remote_fd,
                            ::hostfs_api::OperationId::FIRE_AND_FORGET,
                        ) {
                            ::syslog::warn!(
                                "failed to close orphaned hostfs fd on process exit (pid={:?}, \
                                 remote_fd={}, error={:?})",
                                pid,
                                remote_fd,
                                e
                            );
                        }
                    }
                    for remote_fd in reclaim.orphaned_socket_fds {
                        // Fire-and-forget close of the socket endpoint on networkd: the process is
                        // gone and cannot close it itself, mirroring the hostfs orphan-close above.
                        // networkd's acknowledgement is discarded by the main event loop.
                        if let Err(e) = networkd::send_close_request(remote_fd) {
                            ::syslog::warn!(
                                "failed to close orphaned socket fd on process exit (pid={:?}, \
                                 remote_fd={}, error={:?})",
                                pid,
                                remote_fd,
                                e
                            );
                        }
                    }
                    ::syslog::info!("reclaimed filesystem state (pid={:?})", pid);
                    Ok(false)
                },
                ProcessManagementMessageHeader::Exec => {
                    let exec: ExecMessage = ExecMessage::from_bytes(pm_msg.payload);
                    let pid: ProcessIdentifier = exec.pid;
                    // Exec destroys every thread except the caller, so no console read issued by
                    // the old image may survive into the replacement image.
                    purge_process(assemblers, pid);
                    pending.purge_pid(pid);
                    console_wait.purge_pid(pid);
                    // Bind the VFS to the exec'ing process and seed the root console if this is the
                    // root's first VFS-visible event, mirroring the fork-clone and syscall paths so
                    // close-on-exec is applied against a consistent table. The seed helper is
                    // root-only and idempotent.
                    ::vfs::fd::set_current_process(pid);
                    ::vfs::fd::vfs_seed_root_console(pid);
                    // Drop the process's `FD_CLOEXEC` descriptors, leaving the survivors in place.
                    // Each last-reference drop must fire the same side effects as `close`: a
                    // host-backed handle for which this process held the final reference is closed
                    // on hostfsd, and a pipe end whose count reaches zero wakes its suspended
                    // counterpart (readers see EOF, writers see `EPIPE`). The process stays alive,
                    // so its parked pipe requests are NOT purged.
                    let reclaim: ::vfs::fd::ProcessExitReclaim = ::vfs::fd::vfs_exec_cloexec(pid);
                    for closure in reclaim.pipe_closures {
                        if closure.was_write {
                            handler::pipe::wake_all_readers_eof(closure.pipe_id, pipe_wait);
                        } else {
                            handler::pipe::fail_all_writers_epipe(closure.pipe_id, pipe_wait);
                        }
                    }
                    for remote_fd in reclaim.orphaned_hostfs_fds {
                        // Fire-and-forget close: the descriptor is gone from the exec'd image, so
                        // there is no caller to acknowledge, exactly as on process exit.
                        if let Err(e) = hostfs::send_close_request(
                            remote_fd,
                            ::hostfs_api::OperationId::FIRE_AND_FORGET,
                        ) {
                            ::syslog::warn!(
                                "failed to close orphaned hostfs fd on exec (pid={:?}, \
                                 remote_fd={}, error={:?})",
                                pid,
                                remote_fd,
                                e
                            );
                        }
                    }
                    for remote_fd in reclaim.orphaned_socket_fds {
                        // A `FD_CLOEXEC` socket is released at exec: forward its endpoint close to
                        // networkd, fire-and-forget, mirroring the hostfs orphan-close above.
                        if let Err(e) = networkd::send_close_request(remote_fd) {
                            ::syslog::warn!(
                                "failed to close orphaned socket fd on exec (pid={:?}, \
                                 remote_fd={}, error={:?})",
                                pid,
                                remote_fd,
                                e
                            );
                        }
                    }
                    // Acknowledge the process manager daemon that close-on-exec has been applied, so
                    // it can release the held process. The acknowledgement is necessarily ordered
                    // after the table mutation above, so the released image's cache rebuild observes
                    // the post-close-on-exec table.
                    match exec_ack(
                        ProcessIdentifier::VFSD,
                        ProcessIdentifier::PROCD,
                        pid,
                        ::proc::ExecAckMessage::STATUS_SUCCESS,
                    ) {
                        Ok(ack) => response_context.send(&ack),
                        Err(e) => ::syslog::error!(
                            "failed to build exec acknowledgement (pid={:?}, error={:?})",
                            pid,
                            e
                        ),
                    }
                    ::syslog::info!("applied close-on-exec (pid={:?})", pid);
                    Ok(false)
                },
                _ => {
                    ::syslog::warn!("received unknown process management message, ignoring...");
                    Ok(false)
                },
            }
        },
        SystemMessageHeader::MemoryManagement => {
            ::syslog::warn!("received memory management message, ignoring...");
            Ok(false)
        },
        SystemMessageHeader::FilesystemManagement => {
            ::syslog::warn!("received filesystem management message, ignoring...");
            Ok(false)
        },
    }
}

//==================================================================================================
// IPC Message Dispatch
//==================================================================================================

pub(crate) fn handle_ipc_message(
    message: Message,
    assemblers: &mut BTreeMap<AssemblerKey, AssemblerEntry>,
    pending: &mut PendingQueue,
    console_wait: &mut ConsoleWaitTable,
    pipe_wait: &mut PipeWaitTable,
) -> Result<bool, Error> {
    // The kernel stamps the authoritative originating process and thread into `message.source`.
    // Retain both before parsing shadows the raw message.
    let sender: MessageSender = message.source;
    let source_tid: ThreadIdentifier = sender.tid;
    let source_pid: ProcessIdentifier = sender.pid;

    // Route messages from the process manager daemon (PROCD).
    if source_pid == ProcessIdentifier::PROCD {
        return handle_system_message(message, assemblers, pending, console_wait, pipe_wait);
    }

    // The request identifier lives at a fixed raw offset, so it remains available even when the
    // syscall message itself is malformed.
    let request_id: RequestIdentifier = RequestIdentifier::read_from(&message);
    let response_context: ResponseContext = ResponseContext::new(sender, request_id);

    // Bind the VFS to the requesting process so that descriptor and working-directory operations
    // resolve against its per-process state. `source_pid` is the kernel-attested caller identity
    // (`message.source.pid`; see `caller_pid`), so it is correct even for `fork()` + `execv()`'d
    // children and for requests from non-main threads of a multi-threaded caller. vfsd being
    // single-threaded is what makes mutating this global current-process selector race-free.
    ::vfs::fd::set_current_process(source_pid);

    // Seed the root process's standard console descriptors (0/1/2) if this request is from the
    // root. The helper is root-only and idempotent: a racing child request remains a placeholder,
    // and later children inherit 0/1/2 through `vfs_fork_clone` instead.
    ::vfs::fd::vfs_seed_root_console(source_pid);

    // Parse as SystemCallMessage from user processes.
    let syscall_msg: SystemCallMessage = match SystemCallMessage::try_from_bytes(message.payload) {
        Ok(msg) => msg,
        Err(e) => {
            ::syslog::error!("failed to parse syscall message (error={:?})", e);
            response_context.send(&build_error(source_tid, ErrorCode::InvalidMessage));
            return Ok(false);
        },
    };

    match syscall_msg.kind() {
        //==========================================================================================
        // Short requests: single message request, single message response.
        //==========================================================================================
        SystemCallMessageKind::CloseRequest => {
            if let Some(response) =
                handler::handle_close_with_hostfs(response_context, syscall_msg, pending, pipe_wait)
            {
                response_context.send(&response);
            }
        },
        SystemCallMessageKind::ResolveFdRequest => {
            let response: Message = handler::handle_resolve_fd(source_tid, syscall_msg);
            response_context.send(&response);
        },
        SystemCallMessageKind::ConsoleReadCancelRequest => {
            let request_id: RequestIdentifier = ConsoleReadCancel::target(&syscall_msg.payload);
            let cancelled: bool = console_wait.cancel(source_pid, source_tid, request_id);
            let _ = handler::service_pending_console_input(console_wait);
            response_context.send(&ConsoleReadCancel::build_response(source_tid, cancelled));
        },
        SystemCallMessageKind::PipeOpCancelRequest => {
            let request: PipeOpCancelRequest = PipeOpCancelRequest::from_bytes(syscall_msg.payload);
            // The parked request is keyed by exact caller and request identity. `fd` and operation
            // remain diagnostic. Always acknowledge so the cancelling client cannot be wedged.
            let cancellation: Option<usize> = pipe_wait
                .cancel(source_pid, source_tid, request.request_id())
                .or_else(|| {
                    pending
                        .cancel_read_request(source_pid, source_tid, request.request_id())
                        .then_some(0)
                });
            let transferred: usize = cancellation.unwrap_or(0);
            ::syslog::trace!(
                "cancelled pipe operation (pid={:?}, tid={:?}, fd={}, operation={:?}, \
                 transferred={})",
                source_pid,
                source_tid,
                request.fd(),
                request.operation(),
                transferred
            );
            response_context.send(&PipeOpCancelResponse::build(
                source_tid,
                transferred as u32,
                cancellation.is_some(),
            ));
        },
        SystemCallMessageKind::ConsoleReadRetry => {
            if source_pid == ProcessIdentifier::VFSD {
                handler::retry_console_readers(console_wait);
            } else {
                response_context.send(&build_error(source_tid, ErrorCode::PermissionDenied));
            }
        },
        SystemCallMessageKind::PipeReadRetry => {
            if source_pid == ProcessIdentifier::VFSD {
                let retry: PipeReadRetry = PipeReadRetry::from_bytes(syscall_msg.payload);
                handler::pipe::retry_readers(retry.pipe_id(), pipe_wait);
            } else {
                response_context.send(&build_error(source_tid, ErrorCode::PermissionDenied));
            }
        },
        SystemCallMessageKind::RegisterSocketRequest => {
            let response: Message = handler::handle_register_socket(source_tid, syscall_msg);
            response_context.send(&response);
        },
        SystemCallMessageKind::Dup2Request => {
            let response: Message = handler::handle_dup2(source_tid, syscall_msg, pipe_wait);
            response_context.send(&response);
        },
        SystemCallMessageKind::SeekRequest => {
            if let Some(response) =
                handler::handle_seek_with_hostfs(response_context, syscall_msg, pending)
            {
                response_context.send(&response);
            }
        },
        SystemCallMessageKind::FileSyncRequest => {
            if let Some(response) =
                handler::handle_fsync_with_hostfs(response_context, syscall_msg, pending)
            {
                response_context.send(&response);
            }
        },
        SystemCallMessageKind::FileDataSyncRequest => {
            let response: Message = handler::handle_fdatasync(source_tid, syscall_msg);
            response_context.send(&response);
        },
        SystemCallMessageKind::FileTruncateRequest => {
            if let Some(response) =
                handler::handle_ftruncate_with_hostfs(response_context, syscall_msg, pending)
            {
                response_context.send(&response);
            }
        },
        SystemCallMessageKind::FileSpaceControlRequest => {
            let response: Message = handler::handle_fallocate(source_tid, syscall_msg);
            response_context.send(&response);
        },
        SystemCallMessageKind::FileAdvisoryInformationRequest => {
            let response: Message = handler::handle_fadvise(source_tid, syscall_msg);
            response_context.send(&response);
        },
        SystemCallMessageKind::FileControlRequest => {
            let response: Message = handler::handle_fcntl(source_tid, syscall_msg);
            response_context.send(&response);
        },
        SystemCallMessageKind::FileChmodRequest => {
            if let Some(response) =
                handler::handle_fchmod_with_hostfs(response_context, syscall_msg, pending)
            {
                response_context.send(&response);
            }
        },
        SystemCallMessageKind::FileCreationMaskRequest => {
            let response: Message = handler::handle_umask(source_tid, syscall_msg);
            response_context.send(&response);
        },
        SystemCallMessageKind::FileChownRequest => {
            if let Some(response) =
                handler::handle_fchown_with_hostfs(response_context, syscall_msg, pending)
            {
                response_context.send(&response);
            }
        },
        SystemCallMessageKind::FileChdirRequest => {
            let response: Message = handler::handle_fchdir(source_tid, syscall_msg);
            response_context.send(&response);
        },
        SystemCallMessageKind::UpdateFileAccessTimeRequest => {
            if let Some(response) =
                handler::handle_futimens_with_hostfs(response_context, syscall_msg, pending)
            {
                response_context.send(&response);
            }
        },

        //==========================================================================================
        // Pipe creation: single message request, single message response.
        //==========================================================================================
        SystemCallMessageKind::PipeRequest => {
            let response: Message = handler::pipe::handle_pipe_create(source_tid);
            response_context.send(&response);
        },

        //==========================================================================================
        // Read/Write: single message request + bulk data via push/pull.
        //==========================================================================================
        SystemCallMessageKind::ReadRequest => {
            if let Some(response) = handler::handle_read_with_hostfs(
                response_context,
                syscall_msg,
                pending,
                console_wait,
                pipe_wait,
            ) {
                response_context.send(&response);
            }
        },
        SystemCallMessageKind::WriteRequest => {
            if let Some(response) =
                handler::handle_write_with_hostfs(response_context, syscall_msg, pending, pipe_wait)
            {
                response_context.send(&response);
            }
        },

        //==========================================================================================
        // Terminal control: single message request + termios/winsize via push/pull.
        //==========================================================================================
        SystemCallMessageKind::TtyControlRequest => {
            let response: Message =
                handler::handle_tty_control(source_pid, source_tid, syscall_msg, console_wait);
            response_context.send(&response);
        },

        //==========================================================================================
        // Partial read/write: inline data in message payload.
        //==========================================================================================
        SystemCallMessageKind::PartialReadRequest => {
            let response: Message = handler::handle_pread(source_tid, syscall_msg);
            response_context.send(&response);
        },
        SystemCallMessageKind::PartialWriteRequest => {
            let response: Message = handler::handle_pwrite(source_tid, syscall_msg);
            response_context.send(&response);
        },

        //==========================================================================================
        // Long responses: single request, multi-part response.
        //==========================================================================================
        SystemCallMessageKind::FileStatRequest => {
            if let Some(responses) =
                handler::handle_fstat_with_hostfs(response_context, syscall_msg, pending)
            {
                for response in responses {
                    response_context.send(&response);
                }
            }
        },
        SystemCallMessageKind::GetCurrentWorkingDirectoryRequest => {
            let responses: Vec<Message> = handler::handle_getcwd(source_tid);
            for response in responses {
                response_context.send(&response);
            }
        },
        SystemCallMessageKind::GetDirectoryEntriesRequest => {
            if let Some(responses) =
                handler::handle_getdents_with_hostfs(response_context, syscall_msg, pending)
            {
                for response in responses {
                    response_context.send(&response);
                }
            }
        },

        //==========================================================================================
        // Long requests: multi-part request, single or multi-part response.
        //==========================================================================================
        SystemCallMessageKind::OpenAtRequestPart
        | SystemCallMessageKind::RenameAtRequestPart
        | SystemCallMessageKind::UnlinkAtRequestPart
        | SystemCallMessageKind::FileStatAtRequestPart
        | SystemCallMessageKind::MakeDirectoryAtRequestPart
        | SystemCallMessageKind::ChangeDirectoryRequestPart
        | SystemCallMessageKind::FileAccessAtRequestPart
        | SystemCallMessageKind::SymbolicLinkAtRequestPart
        | SystemCallMessageKind::LinkAtRequestPart
        | SystemCallMessageKind::ReadLinkAtRequestPart
        | SystemCallMessageKind::UpdateFileAccessTimeAtRequestPart
        | SystemCallMessageKind::FileChownAtRequestPart
        | SystemCallMessageKind::FileChmodAtRequestPart
        | SystemCallMessageKind::HostMountRequestPart
        | SystemCallMessageKind::HostUmountRequestPart
        | SystemCallMessageKind::PollRequestPart => {
            let part: SystemCallMessagePart =
                SystemCallMessagePart::from_bytes(syscall_msg.payload);
            if let Some((response_context, responses)) = assemble_and_dispatch(
                response_context,
                syscall_msg.kind(),
                part,
                assemblers,
                pending,
                console_wait,
            ) {
                for response in responses {
                    response_context.send(&response);
                }
            }
        },

        //==========================================================================================
        // Unknown or unsupported headers.
        //==========================================================================================
        _ => {
            let hdr = syscall_msg.kind();
            ::syslog::warn!("received unsupported syscall header: {:?}", hdr);
            response_context.send(&build_error(source_tid, ErrorCode::InvalidMessage));
        },
    }

    Ok(false)
}
