// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![no_std]
#![no_main]

//==================================================================================================
// Modules
//==================================================================================================

mod assembler;
mod console_wait;
mod error;
mod handler;
mod hostfs;
mod init;
mod ipc;
mod networkd;
mod pending;
mod pipe_wait;

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;
extern crate libc_string;
extern crate nvx;
extern crate nvx_crt0;

use ::proc::{
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
    SignupResponseMessage,
};
use ::sys::{
    error::ErrorCode,
    ipc::{
        Message,
        MessageType,
        SystemMessage,
        SystemMessageHeader,
    },
    pm::ProcessIdentifier,
};
use ::syscall::SystemCallMessageKind;
use alloc::collections::{
    BTreeMap,
    VecDeque,
};

//==================================================================================================
// Main Function
//==================================================================================================

#[unsafe(no_mangle)]
pub fn main() {
    let mypid: ProcessIdentifier = match ::sys::kcall::pm::getpid() {
        Ok(pid) => pid,
        Err(e) => panic!("failed to get pid (error={:?})", e),
    };
    let myname: &str = ::config::daemons::VFSD_NAME;
    let tid: ::sys::pm::ThreadIdentifier =
        ::sys::kcall::pm::__kcall_gettid().expect("failed to get thread identifier");

    ::syslog::info!("running virtual file system daemon (pid={:?})...", mypid);

    // Initialize VFS and mount the RAMFS image.
    init::vfs_init_ramfs();

    // Signup to the process manager daemon.
    // Because the kernel spawns all multibinary processes concurrently, user processes may send
    // IPC messages to vfsd before vfsd finishes its signup handshake with procd. We handle
    // this by buffering any IPC messages received during the signup phase.
    let mut buffered_messages: VecDeque<Message> = VecDeque::new();
    {
        let token: ::sys::ipc::RequestToken =
            ::sys::ipc::RequestToken::allocate(tid, ProcessIdentifier::PROCD)
                .expect("failed to allocate signup request identifier");
        let mut message: Message =
            ::proc::signup_request(mypid, myname).expect("failed to build signup request");
        token.identifier().write_to(&mut message);
        ::sys::kcall::ipc::__kcall_send(&message).expect("failed to send signup request");

        // Wait for the signup response, buffering any interleaved IPC messages.
        loop {
            let message: Message =
                ::sys::kcall::ipc::__kcall_recv().expect("failed to receive signup response");
            let source: ::sys::ipc::MessageSender = message.source;
            let request_id: ::sys::ipc::RequestIdentifier =
                ::sys::ipc::RequestIdentifier::read_from(&message);
            if source.pid == ProcessIdentifier::PROCD && request_id == token.identifier() {
                assert_eq!(message.message_type, MessageType::Ipc, "invalid signup response type");
                let sys_msg: SystemMessage =
                    SystemMessage::from_bytes(message.payload).expect("invalid signup response");
                assert!(
                    matches!(sys_msg.header, SystemMessageHeader::ProcessManagement),
                    "invalid signup system message"
                );
                let pm_msg: ProcessManagementMessage =
                    ProcessManagementMessage::from_bytes(sys_msg.payload)
                        .expect("invalid signup process-management message");
                assert!(
                    matches!(pm_msg.header, ProcessManagementMessageHeader::SignupResponse),
                    "unexpected signup response"
                );
                let resp: SignupResponseMessage = SignupResponseMessage::from_bytes(pm_msg.payload);
                let status: i32 = resp.status;
                if status != 0 {
                    panic!("signup failed (status={})", status);
                }
                ::syslog::info!("signed up with procd");
                break;
            }

            if message.message_type == MessageType::Ipc {
                // Not a signup response -- buffer it for later processing.
                ::syslog::info!("buffered IPC message during signup");
                buffered_messages.push_back(message);
            } else {
                // Non-IPC messages during signup are discarded.
                ::syslog::error!(
                    "discarding non-IPC message during signup (type={:?})",
                    message.message_type
                );
            }
        }
    }

    let subscription: Message =
        ::syscall::poll::input_message::PollInputRequest::build_subscription(tid);
    ::sys::kcall::ipc::__kcall_send(&subscription)
        .expect("failed to subscribe to console input notifications");

    // Bounded multi-part request assembler map keyed by exact caller, header, and request ID.
    let mut assemblers: BTreeMap<assembler::AssemblerKey, assembler::AssemblerEntry> =
        BTreeMap::new();

    // Pending hostfs operations awaiting IKC responses.
    let mut pending: pending::PendingQueue = pending::PendingQueue::new();

    // Suspended pipe readers/writers awaiting their complementary operation.
    let mut pipe_wait: pipe_wait::PipeWaitTable = pipe_wait::PipeWaitTable::new();

    // Suspended console readers awaiting cooked input or end-of-file.
    let mut console_wait: console_wait::ConsoleWaitTable = console_wait::ConsoleWaitTable::new();

    // In-flight multi-part hostfs *response* assembler, paired with the op_id
    // extracted eagerly from part 0 so a discarded stream can be cancelled (the
    // pending op would otherwise sit until the eviction timer fires).
    //
    // Long-target `readlink` returns its response as a stream of
    // `HostFsReadlinkResponsePart` messages. Because hostfsd's worker is single-
    // threaded and replies to one request at a time, at most one such stream is in
    // flight at any moment, so a single slot suffices. If a fresh `part_number == 0`
    // arrives while another stream is still being assembled, the old stream is
    // discarded and its caller is failed with `IoErr`.
    //
    // TODO: if hostfsd ever becomes multi-stream (e.g., a worker pool that interleaves
    // long responses), replace this single-slot assembler with an op_id-keyed map so
    // concurrent response streams can be assembled independently.
    let mut readlink_response_asm: Option<(
        ::syscall::message::SystemCallLongMessage,
        ::hostfs_api::OperationId,
    )> = None;

    // In-flight multi-part hostfs *readdir* response assembler. A directory entry whose
    // name exceeds the inline `ReadDirEntry` capacity is returned as a stream of
    // `HostFsReadDirResponsePart` messages. As with the readlink assembler above,
    // hostfsd's single-threaded worker guarantees at most one multi-part response stream
    // is in flight at any moment, so a single slot suffices.
    let mut readdir_response_asm: Option<(
        ::syscall::message::SystemCallLongMessage,
        ::hostfs_api::OperationId,
    )> = None;

    // Process any messages that were buffered during the signup phase.
    while let Some(message) = buffered_messages.pop_front() {
        match ipc::handle_ipc_message(
            message,
            &mut assemblers,
            &mut pending,
            &mut console_wait,
            &mut pipe_wait,
        ) {
            Ok(true) => {
                let e = ::sys::kcall::pm::__kcall_exit(0);
                ::syslog::error!("failed to shutdown vfsd (error={:?})", e);
                loop {
                    ::core::hint::spin_loop();
                }
            },
            Ok(false) => continue,
            Err(e) => ::syslog::error!("failed to handle buffered ipc request (error={:?})", e),
        }
    }

    // Single event loop: processes both IPC messages from guest apps and IKC responses
    // from hostfsd without nesting or blocking waits.
    loop {
        match ::sys::kcall::ipc::__kcall_recv() {
            Ok(message) => match message.message_type {
                MessageType::Ipc => {
                    match ipc::handle_ipc_message(
                        message,
                        &mut assemblers,
                        &mut pending,
                        &mut console_wait,
                        &mut pipe_wait,
                    ) {
                        Ok(true) => break,
                        Ok(false) => {},
                        Err(e) => {
                            ::syslog::error!("failed to handle ipc request (error={:?})", e)
                        },
                    }
                },
                MessageType::Ikc => {
                    // Check if this is a hostfs response for a pending operation.
                    if let Ok(syscall_msg) =
                        ::syscall::SystemCallMessage::try_from_bytes(message.payload)
                    {
                        let header: SystemCallMessageKind = syscall_msg.kind();
                        if header == SystemCallMessageKind::ConsoleInputAvailable {
                            let source: ::sys::ipc::MessageSender = message.source;
                            if source != ::sys::ipc::MessageSender::KERNEL {
                                ::syslog::warn!(
                                    "ignored console input notification from {:?}",
                                    source
                                );
                                continue;
                            }
                            handler::handle_console_input_available(&mut console_wait);
                            continue;
                        }
                        // networkd acknowledges a forwarded socket-endpoint close with an IKC
                        // `CloseResponse`. That close is fire-and-forget — vfsd does not wait on it
                        // — so the acknowledgement is expected and silently discarded.
                        if header == SystemCallMessageKind::CloseResponse {
                            continue;
                        }
                        // Console echo writes deliberately leave their `WriteResponse`
                        // acknowledgement for this event loop; consuming it in the helper with a
                        // nested receive could dequeue an unrelated guest request.
                        if header == SystemCallMessageKind::WriteResponse {
                            continue;
                        }
                        // Multi-part response stream: assemble parts before dispatch.
                        // The outer request-ID field echoes op_id, and the assembled body retains
                        // the same value for compatibility with the hostfs response decoder.
                        if header == SystemCallMessageKind::HostFsReadlinkResponsePart {
                            let outer_op_id: ::hostfs_api::OperationId =
                                ::hostfs_api::OperationId::from_le_bytes(
                                    syscall_msg.request_id().raw().to_le_bytes(),
                                );
                            let part: ::syscall::message::SystemCallMessagePart =
                                ::syscall::message::SystemCallMessagePart::from_bytes(
                                    syscall_msg.payload,
                                );
                            if let Some((body, op_id)) = pending::accumulate_response_part(
                                &mut readlink_response_asm,
                                &mut pending,
                                part,
                                outer_op_id,
                                pending::LongResponseStream::Readlink,
                            ) {
                                // op_id is known from part 0; the body still carries it
                                // in bytes [0..4] for `complete_readlink_long`.
                                if let Some(op) = pending.remove(op_id) {
                                    pending::complete_readlink_long(op, &body);
                                } else if pending.discard_abandoned_operation(op_id) {
                                    // The originating process exited or exec'd while the multipart
                                    // response was in flight.
                                } else {
                                    ::syslog::warn!(
                                        "long readlink response with no pending op (op_id={})",
                                        op_id,
                                    );
                                }
                            }
                            continue;
                        }
                        if header == SystemCallMessageKind::HostFsReadDirResponsePart {
                            let outer_op_id: ::hostfs_api::OperationId =
                                ::hostfs_api::OperationId::from_le_bytes(
                                    syscall_msg.request_id().raw().to_le_bytes(),
                                );
                            let part: ::syscall::message::SystemCallMessagePart =
                                ::syscall::message::SystemCallMessagePart::from_bytes(
                                    syscall_msg.payload,
                                );
                            if let Some((body, op_id)) = pending::accumulate_response_part(
                                &mut readdir_response_asm,
                                &mut pending,
                                part,
                                outer_op_id,
                                pending::LongResponseStream::ReadDir,
                            ) {
                                // Decode the long directory entry and fold it into the
                                // in-progress getdents sweep, then advance the sweep
                                // (request the next entry or send the final response).
                                if pending.discard_abandoned_operation(op_id) {
                                    continue;
                                }
                                let decoded =
                                    ::hostfs_api::long_msg::deserialize_long_readdir_response(
                                        &body,
                                    );
                                let step: Option<pending::GetdentsStep> =
                                    match (decoded, pending.get_mut(op_id)) {
                                        (Some(entry), Some(op))
                                            if matches!(
                                                op.kind,
                                                pending::PendingOpKind::Getdents { .. }
                                            ) =>
                                        {
                                            Some(pending::push_getdents_entry(
                                                op,
                                                entry.name,
                                                entry.is_dir,
                                            ))
                                        },
                                        (None, _) => {
                                            ::syslog::error!(
                                                "failed to deserialize long readdir response \
                                                 (op_id={}, body_len={})",
                                                op_id,
                                                body.len(),
                                            );
                                            if let Some(op) = pending.remove(op_id) {
                                                pending::cancel_pending_op(op, ErrorCode::IoErr);
                                            }
                                            None
                                        },
                                        (Some(_), Some(_)) => {
                                            // Deserialized cleanly, but the buffered op is
                                            // not a getdents sweep — a protocol desync. Fail
                                            // the caller now rather than leaving it hung.
                                            ::syslog::error!(
                                                "long readdir response for non-getdents op \
                                                 (op_id={})",
                                                op_id,
                                            );
                                            if let Some(op) = pending.remove(op_id) {
                                                pending::cancel_pending_op(op, ErrorCode::IoErr);
                                            }
                                            None
                                        },
                                        (Some(_), None) => {
                                            ::syslog::warn!(
                                                "long readdir response with no matching getdents \
                                                 op (op_id={})",
                                                op_id,
                                            );
                                            None
                                        },
                                    };
                                if let Some(step) = step {
                                    pending::drive_getdents(&mut pending, op_id, step);
                                }
                            }
                            continue;
                        }
                        if matches!(
                            header,
                            SystemCallMessageKind::HostFsStatResponse
                                | SystemCallMessageKind::HostFsLstatResponse
                                | SystemCallMessageKind::HostFsPathStatResponse
                        ) {
                            let op_id: ::hostfs_api::OperationId =
                                ::hostfs_api::get_op_id(&message.payload);
                            let step: Option<pending::StatMetadataStep> = pending
                                .get_mut(op_id)
                                .map(|op| pending::stage_stat_metadata(op, &message.payload));
                            match step {
                                Some(pending::StatMetadataStep::Wait) => continue,
                                Some(pending::StatMetadataStep::Complete) => {
                                    if let Some(op) = pending.remove(op_id) {
                                        pending::complete_pending_op(op, &message.payload);
                                    }
                                    continue;
                                },
                                Some(pending::StatMetadataStep::Invalid) => {
                                    if let Some(op) = pending.remove(op_id) {
                                        pending::cancel_pending_op(op, ErrorCode::IoErr);
                                    }
                                    continue;
                                },
                                None => {},
                            }
                        }
                        if header == SystemCallMessageKind::HostFsStatTimesResponse {
                            let op_id: ::hostfs_api::OperationId =
                                ::hostfs_api::get_op_id(&message.payload);
                            let is_staged_stat: bool = pending.get_mut(op_id).is_some_and(|op| {
                                matches!(
                                    op.kind,
                                    pending::PendingOpKind::StatTimes { .. }
                                        | pending::PendingOpKind::LstatTimes { .. }
                                        | pending::PendingOpKind::PathStatTimes { .. }
                                        | pending::PendingOpKind::ChdirTimes { .. }
                                )
                            });
                            if is_staged_stat {
                                if let Some(op) = pending.remove(op_id) {
                                    pending::complete_stat_times(op, &message.payload);
                                }
                            } else if let Some(op) = pending.remove(op_id) {
                                ::syslog::error!(
                                    "stat timestamps for non-staged operation (op_id={})",
                                    op_id,
                                );
                                pending::cancel_pending_op(op, ErrorCode::IoErr);
                            } else if pending.discard_abandoned_operation(op_id) {
                                // The originating process exited while stat was in flight.
                            } else {
                                ::syslog::warn!(
                                    "stat timestamps with no pending op (op_id={})",
                                    op_id,
                                );
                            }
                            continue;
                        }
                        if header.is_hostfs_response() {
                            let op_id: ::hostfs_api::OperationId =
                                ::hostfs_api::get_op_id(&message.payload);
                            // Getdents over hostfs is an async sweep: one readdir entry
                            // per round-trip. Keep the op buffered and re-arm another
                            // request until the directory is exhausted or the requested
                            // entry count is reached.
                            if header == SystemCallMessageKind::HostFsReadDirResponse {
                                let step: Option<pending::GetdentsStep> =
                                    match pending.get_mut(op_id) {
                                        Some(op)
                                            if matches!(
                                                op.kind,
                                                pending::PendingOpKind::Getdents { .. }
                                            ) =>
                                        {
                                            Some(pending::step_getdents(op, &message.payload))
                                        },
                                        _ => None,
                                    };
                                if let Some(step) = step {
                                    pending::drive_getdents(&mut pending, op_id, step);
                                    continue;
                                }
                            }
                            if let Some(op) = pending.remove(op_id) {
                                pending::complete_pending_op(op, &message.payload);
                            } else if pending.complete_abandoned_operation(op_id, &message.payload)
                            {
                                // The originating process exited or exec'd before hostfsd replied.
                                // Any returned remote handle was closed without recreating VFS state.
                            } else if op_id == ::hostfs_api::OperationId::FIRE_AND_FORGET {
                                // Expected: response to a fire-and-forget request (e.g. a
                                // best-effort close on process exit or open-failure cleanup) for
                                // which no pending op was registered. Discard it silently.
                            } else {
                                ::syslog::warn!(
                                    "hostfs response with no pending op (header={:?}, op_id={})",
                                    header,
                                    op_id,
                                );
                            }
                            continue;
                        }
                    }
                    ::syslog::warn!("received unexpected ikc message, ignoring");
                },
                MessageType::Interrupt => {
                    ::syslog::warn!("received unexpected interrupt, ignoring");
                },
                MessageType::Exception => {
                    ::syslog::warn!("received unexpected exception, ignoring");
                },
                MessageType::ProcessTerminationEvent => {
                    ::syslog::warn!("received unexpected process termination event, ignoring");
                },
                MessageType::ProcessCreationEvent => {
                    ::syslog::warn!("received unexpected process creation event, ignoring");
                },
                MessageType::ThreadTerminationEvent => {
                    ::syslog::warn!("received unexpected thread termination event, ignoring");
                },
                MessageType::PullResponse => {
                    ::syslog::warn!("received unexpected pull response, ignoring");
                },
            },
            Err(e) => ::syslog::error!("failed to receive message (error={:?})", e),
        }
    }

    // Shutdown VFS daemon.
    let e = ::sys::kcall::pm::__kcall_exit(0);
    ::syslog::error!("failed to shutdown vfsd (error={:?})", e);

    loop {
        ::core::hint::spin_loop();
    }
}
