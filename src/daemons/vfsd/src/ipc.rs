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
    pending::PendingQueue,
};
use ::proc::{
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
/// route the reply), so this almost always takes the TID branch. The owning process cannot be
/// recovered locally — PIDs and TIDs are drawn from independent counters and a process may be
/// multi-threaded — so the authoritative owner is obtained from the kernel via
/// [`__kcall_getpid_from_tid`](::sys::kcall::pm::__kcall_getpid_from_tid). This keys the caller's
/// per-process VFS state (fd table and cwd) by the correct process even for requests issued from a
/// non-main thread of a multi-threaded process.
///
/// A PID-encoded source (e.g. a message routed with a `ProcessIdentifier`, such as procd control
/// messages) is already a process identifier and is returned as-is.
///
/// If the kernel lookup fails (which should not happen while the caller is blocked awaiting the
/// reply), this falls back to deriving the PID from the raw TID value so the daemon stays live; the
/// fallback is only ever correct for single-threaded callers where TID == PID.
fn caller_pid(message: &Message) -> ProcessIdentifier {
    let sender: MessageSender = message.source;
    match sender.as_id() {
        Ok(pid) => pid,
        Err(tid) => resolve_owning_pid(tid),
    }
}

/// Resolves the process that owns `tid`, asking the kernel for the authoritative mapping.
///
/// The caller of a synchronous filesystem syscall stays blocked awaiting vfsd's reply, so its
/// thread is guaranteed to be alive here and the kernel lookup succeeds. The TID-cast fallback is a
/// defensive last resort that keeps vfsd live if the lookup ever fails; it is only correct for
/// single-threaded callers where TID == PID.
fn resolve_owning_pid(tid: ThreadIdentifier) -> ProcessIdentifier {
    match ::sys::kcall::pm::__kcall_getpid_from_tid(tid) {
        Ok(pid) => pid,
        Err(e) => {
            ::syslog::error!(
                "failed to resolve owning process of caller thread (tid={:?}, error={:?}); this \
                 should not happen during normal operation (the caller stays blocked awaiting the \
                 reply); falling back to TID-derived PID, which is correct only when TID == PID",
                tid,
                e
            );
            ProcessIdentifier::from(i32::from(tid))
        },
    }
}

//==================================================================================================
// SystemMessage Handler (procd shutdown)
//==================================================================================================

fn handle_system_message(message: Message) -> Result<bool, Error> {
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
                    // open file descriptors so that surviving siblings retain correct last-reference
                    // accounting. Any host-backed descriptors for which the process held the final
                    // reference can no longer be closed by the process itself, so close them on
                    // hostfsd here to avoid leaking remote handles.
                    let orphaned: Vec<i32> = ::vfs::fd::vfs_process_exit(pid);
                    for remote_fd in orphaned {
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
                    ::syslog::info!("reclaimed filesystem state (pid={:?})", pid);
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
) -> Result<bool, Error> {
    let source_tid: ThreadIdentifier = caller_tid(&message);
    let source_pid: ProcessIdentifier = caller_pid(&message);

    // Route messages from the process manager daemon (PROCD).
    if source_pid == ProcessIdentifier::PROCD {
        return handle_system_message(message);
    }

    // Bind the VFS to the requesting process so that descriptor and working-directory operations
    // resolve against its per-process state. Guest syscall messages encode their source as the
    // caller's thread id, so `source_pid` is the authoritative owner of that thread as reported by
    // the kernel (see `caller_pid`/`resolve_owning_pid`). This is correct even for a request issued
    // from a non-main thread of a multi-threaded process. vfsd being single-threaded is what makes
    // mutating this global current-process selector race-free.
    ::vfs::fd::set_current_process(source_pid);

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
                handler::handle_close_with_hostfs(source_tid, syscall_msg, pending)
            {
                send_response(&response);
            }
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
        // Read/Write: single message request + bulk data via push/pull.
        //==========================================================================================
        SystemCallMessageHeader::ReadRequest => {
            if let Some(response) =
                handler::handle_read_with_hostfs(source_pid, source_tid, syscall_msg, pending)
            {
                send_response(&response);
            }
        },
        SystemCallMessageHeader::WriteRequest => {
            if let Some(response) =
                handler::handle_write_with_hostfs(source_pid, source_tid, syscall_msg, pending)
            {
                send_response(&response);
            }
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
                handler::handle_getdents_with_hostfs(source_pid, source_tid, syscall_msg, pending)
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
            if let Some(responses) = assemble_and_dispatch(
                source_pid,
                source_tid,
                syscall_msg.header,
                part,
                assemblers,
                pending,
            ) {
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
