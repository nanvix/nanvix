// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    assembler::{
        assemble_and_dispatch,
        AssemblerEntry,
    },
    error::{
        build_error,
        send_response,
    },
    handler,
    hostfs,
    networkd,
    pending::PendingQueue,
    pipe_wait::PipeWaitTable,
};
use ::proc::{
    exec_ack,
    ExecMessage,
    ForkCloneMessage,
    ProcessExitMessage,
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
    ShutdownMessage,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
        MessageSender,
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
    SystemCallMessage,
    SystemCallMessageHeader,
};
use alloc::{
    collections::BTreeMap,
    vec::Vec,
};

//==================================================================================================
// Helpers: Extract caller identity from message source
//==================================================================================================

/// Extracts the caller's thread identifier from an IPC message.
///
/// Guest syscall requests encode their source as the caller's thread id (`MessageSender::from` a
/// `ThreadIdentifier`) because vfsd needs the TID to route the reply, so this almost always takes
/// the TID branch and returns it as-is. A PID-encoded source (e.g. a message routed with a
/// `ProcessIdentifier`) is handled by deriving a TID from the PID value, which is correct only for
/// single-threaded processes where TID == PID.
fn caller_tid(message: &Message) -> ThreadIdentifier {
    let source = message.source;
    match source.as_id() {
        Ok(pid) => {
            // PID-encoded source — derive TID from PID value (valid for single-threaded callers).
            ThreadIdentifier::from(i32::from(pid))
        },
        Err(tid) => tid,
    }
}

/// Extracts the caller's process identifier from an IPC message.
///
/// Guest syscall requests encode their source as the caller's *thread* id (vfsd needs the TID to
/// route the reply), so this almost always takes the TID branch and derives the PID by casting the
/// TID value. That cast resolves to the correct process only for single-threaded callers where
/// TID == PID; a request issued from a non-main thread of a multi-threaded process would be
/// misattributed, keying its per-process VFS state (fd table and cwd) by the wrong identifier.
///
/// TODO(#2529): derive the caller PID from an authoritative value supplied by the kernel (e.g. a
/// PID carried alongside the TID in the IPC metadata) instead of casting the TID, so
/// multi-threaded callers are attributed to the correct process.
fn caller_pid(message: &Message) -> ProcessIdentifier {
    let sender: MessageSender = message.source;
    match sender.as_id() {
        Ok(pid) => pid,
        Err(tid) => ProcessIdentifier::from(i32::from(tid)),
    }
}

//==================================================================================================
// SystemMessage Handler (procd shutdown)
//==================================================================================================

fn handle_system_message(message: Message, pipe_wait: &mut PipeWaitTable) -> Result<bool, Error> {
    // State-mutating process-management messages are privileged: only procd may direct them. The
    // caller (handle_ipc_message) routes here only when the message source is procd, which is the
    // runtime gate. Note that this trusts `message.source`: the kernel currently only *logs* an
    // invalid source and still delivers the message (see `src/kernel/src/ipc/send.rs`), so it does
    // not by itself stop another process from spoofing a procd source. Robustly closing that gap
    // would require the kernel to reject messages whose source does not match the real sender,
    // tracked in issue #2527. The debug-assert below is a development sanity check of the routing
    // invariant, not a security control.
    debug_assert_eq!(
        caller_pid(&message),
        ProcessIdentifier::PROCD,
        "handle_system_message invoked with non-procd source"
    );
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
                    if let Err(e) = ::vfs::fd::vfs_fork_clone(parent, child) {
                        ::syslog::error!(
                            "failed to clone filesystem state (parent={:?}, child={:?}, \
                             error={:?})",
                            parent,
                            child,
                            e
                        );
                    } else {
                        ::syslog::info!(
                            "cloned filesystem state (parent={:?}, child={:?})",
                            parent,
                            child
                        );
                    }
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
                        Ok(ack) => send_response(&ack),
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
    assemblers: &mut BTreeMap<(i32, u16), AssemblerEntry>,
    pending: &mut PendingQueue,
    pipe_wait: &mut PipeWaitTable,
) -> Result<bool, Error> {
    let source_tid: ThreadIdentifier = caller_tid(&message);
    let source_pid: ProcessIdentifier = caller_pid(&message);

    // Route messages from the process manager daemon (PROCD).
    if source_pid == ProcessIdentifier::PROCD {
        return handle_system_message(message, pipe_wait);
    }

    // Bind the VFS to the requesting process so that descriptor and working-directory operations
    // resolve against its per-process state. Guest syscall messages encode their source as the
    // caller's thread id, so `source_pid` is obtained by casting that TID to a PID (see
    // `caller_pid`). This resolves to the correct process only for single-threaded callers where
    // TID == PID; a request from a non-main thread of a multi-threaded process would be
    // misattributed (see the TODO in `caller_pid` for the authoritative-PID fix). vfsd being
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
            send_response(&build_error(source_tid, ErrorCode::InvalidMessage));
            return Ok(false);
        },
    };

    match syscall_msg.header {
        //==========================================================================================
        // Short requests: single message request, single message response.
        //==========================================================================================
        SystemCallMessageHeader::CloseRequest => {
            if let Some(response) =
                handler::handle_close_with_hostfs(source_tid, syscall_msg, pending, pipe_wait)
            {
                send_response(&response);
            }
        },
        SystemCallMessageHeader::ResolveFdRequest => {
            let response: Message = handler::handle_resolve_fd(source_tid, syscall_msg);
            send_response(&response);
        },
        SystemCallMessageHeader::RegisterSocketRequest => {
            let response: Message = handler::handle_register_socket(source_tid, syscall_msg);
            send_response(&response);
        },
        SystemCallMessageHeader::Dup2Request => {
            let response: Message = handler::handle_dup2(source_tid, syscall_msg, pipe_wait);
            send_response(&response);
        },
        SystemCallMessageHeader::SeekRequest => {
            if let Some(response) =
                handler::handle_seek_with_hostfs(source_tid, syscall_msg, pending)
            {
                send_response(&response);
            }
        },
        SystemCallMessageHeader::FileSyncRequest => {
            if let Some(response) =
                handler::handle_fsync_with_hostfs(source_tid, syscall_msg, pending)
            {
                send_response(&response);
            }
        },
        SystemCallMessageHeader::FileDataSyncRequest => {
            let response: Message = handler::handle_fdatasync(source_tid, syscall_msg);
            send_response(&response);
        },
        SystemCallMessageHeader::FileTruncateRequest => {
            if let Some(response) =
                handler::handle_ftruncate_with_hostfs(source_tid, syscall_msg, pending)
            {
                send_response(&response);
            }
        },
        SystemCallMessageHeader::FileSpaceControlRequest => {
            let response: Message = handler::handle_fallocate(source_tid, syscall_msg);
            send_response(&response);
        },
        SystemCallMessageHeader::FileAdvisoryInformationRequest => {
            let response: Message = handler::handle_fadvise(source_tid, syscall_msg);
            send_response(&response);
        },
        SystemCallMessageHeader::FileControlRequest => {
            let response: Message = handler::handle_fcntl(source_tid, syscall_msg);
            send_response(&response);
        },
        SystemCallMessageHeader::FileChmodRequest => {
            let response: Message = handler::handle_fchmod(source_tid, syscall_msg);
            send_response(&response);
        },
        SystemCallMessageHeader::FileChownRequest => {
            let response: Message = handler::handle_fchown(source_tid, syscall_msg);
            send_response(&response);
        },
        SystemCallMessageHeader::FileChdirRequest => {
            let response: Message = handler::handle_fchdir(source_tid, syscall_msg);
            send_response(&response);
        },
        SystemCallMessageHeader::UpdateFileAccessTimeRequest => {
            let response: Message = handler::handle_futimens(source_tid, syscall_msg);
            send_response(&response);
        },

        //==========================================================================================
        // Pipe creation: single message request, single message response.
        //==========================================================================================
        SystemCallMessageHeader::PipeRequest => {
            let response: Message = handler::pipe::handle_pipe_create(source_tid);
            send_response(&response);
        },

        //==========================================================================================
        // Read/Write: single message request + bulk data via push/pull.
        //==========================================================================================
        SystemCallMessageHeader::ReadRequest => {
            if let Some(response) = handler::handle_read_with_hostfs(
                source_pid,
                source_tid,
                syscall_msg,
                pending,
                pipe_wait,
            ) {
                send_response(&response);
            }
        },
        SystemCallMessageHeader::WriteRequest => {
            if let Some(response) = handler::handle_write_with_hostfs(
                source_pid,
                source_tid,
                syscall_msg,
                pending,
                pipe_wait,
            ) {
                send_response(&response);
            }
        },

        //==========================================================================================
        // Terminal control: single message request + termios/winsize via push/pull.
        //==========================================================================================
        SystemCallMessageHeader::TtyControlRequest => {
            let response: Message =
                handler::handle_tty_control(source_pid, source_tid, syscall_msg);
            send_response(&response);
        },

        //==========================================================================================
        // Partial read/write: inline data in message payload.
        //==========================================================================================
        SystemCallMessageHeader::PartialReadRequest => {
            let response: Message = handler::handle_pread(source_tid, syscall_msg);
            send_response(&response);
        },
        SystemCallMessageHeader::PartialWriteRequest => {
            let response: Message = handler::handle_pwrite(source_tid, syscall_msg);
            send_response(&response);
        },

        //==========================================================================================
        // Long responses: single request, multi-part response.
        //==========================================================================================
        SystemCallMessageHeader::FileStatRequest => {
            if let Some(responses) =
                handler::handle_fstat_with_hostfs(source_tid, syscall_msg, pending)
            {
                for response in responses {
                    send_response(&response);
                }
            }
        },
        SystemCallMessageHeader::GetCurrentWorkingDirectoryRequest => {
            let responses: Vec<Message> = handler::handle_getcwd(source_tid);
            for response in responses {
                send_response(&response);
            }
        },
        SystemCallMessageHeader::GetDirectoryEntriesRequest => {
            if let Some(responses) =
                handler::handle_getdents_with_hostfs(source_tid, syscall_msg, pending)
            {
                for response in responses {
                    send_response(&response);
                }
            }
        },

        //==========================================================================================
        // Long requests: multi-part request, single or multi-part response.
        //==========================================================================================
        SystemCallMessageHeader::OpenAtRequestPart
        | SystemCallMessageHeader::RenameAtRequestPart
        | SystemCallMessageHeader::UnlinkAtRequestPart
        | SystemCallMessageHeader::FileStatAtRequestPart
        | SystemCallMessageHeader::MakeDirectoryAtRequestPart
        | SystemCallMessageHeader::ChangeDirectoryRequestPart
        | SystemCallMessageHeader::FileAccessAtRequestPart
        | SystemCallMessageHeader::SymbolicLinkAtRequestPart
        | SystemCallMessageHeader::LinkAtRequestPart
        | SystemCallMessageHeader::ReadLinkAtRequestPart
        | SystemCallMessageHeader::UpdateFileAccessTimeAtRequestPart
        | SystemCallMessageHeader::FileChownAtRequestPart
        | SystemCallMessageHeader::FileChmodAtRequestPart
        | SystemCallMessageHeader::HostMountRequestPart
        | SystemCallMessageHeader::HostUmountRequestPart => {
            let part: SystemCallMessagePart =
                SystemCallMessagePart::from_bytes(syscall_msg.payload);
            if let Some(responses) =
                assemble_and_dispatch(source_tid, syscall_msg.header, part, assemblers, pending)
            {
                for response in responses {
                    send_response(&response);
                }
            }
        },

        //==========================================================================================
        // Unknown or unsupported headers.
        //==========================================================================================
        _ => {
            let hdr = syscall_msg.header;
            ::syslog::warn!("received unsupported syscall header: {:?}", hdr);
            send_response(&build_error(source_tid, ErrorCode::InvalidMessage));
        },
    }

    Ok(false)
}
