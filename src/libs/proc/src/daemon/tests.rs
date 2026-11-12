// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use crate::WaitResponseMessage;
use ::core::mem::ManuallyDrop;
use ::sys::ExitStatus;

/// Test stand-in for the process-exit notification: accepts and discards the message.
pub(super) fn send_process_exit_notification(_request: &Message) -> Result<(), Error> {
    Ok(())
}

/// Test stand-in for the terminal-detach notification: accepts and discards the message.
pub(super) fn send_terminal_detach_notification(_request: &Message) -> Result<(), Error> {
    Ok(())
}

fn process_identity(uid: usize) -> ProcessIdentity {
    ProcessIdentity::new(UserIdentifier::from(uid), GroupIdentifier::ROOT)
}

fn process_record(pid: ProcessIdentifier, identity: Option<ProcessIdentity>) -> ProcessRecord {
    ProcessRecord {
        name: String::new(),
        identity,
        parent: None,
        children: Vec::new(),
        fork_clone_done: false,
        fork_clone_failed: false,
        fork_clone_request_id: None,
        zombie: None,
        sid: pid,
        pgid: pid,
    }
}

fn process_daemon(
    records: &[(ProcessIdentifier, Option<ProcessIdentity>)],
) -> ManuallyDrop<ProcessDaemon> {
    let mut processes: BTreeMap<ProcessIdentifier, ProcessRecord> = BTreeMap::new();
    for (pid, identity) in records.iter() {
        processes.insert(*pid, process_record(*pid, identity.clone()));
    }

    ManuallyDrop::new(ProcessDaemon {
        processes,
        init_proc: None,
        pending_fork_syncs: Vec::new(),
        pending_execs: Vec::new(),
        next_request_id: 1,
        pending_setsid: Vec::new(),
        blocked: Vec::new(),
        controlling_terminal: ControllingTerminal::Unassigned,
    })
}

fn response_context(process: ProcessIdentifier) -> ResponseContext {
    ResponseContext::new(
        MessageSender::new(process, ThreadIdentifier::from(20)),
        RequestIdentifier::from_raw(99),
    )
}

fn assign_controlling_terminal(
    daemon: &mut ProcessDaemon,
    session: ProcessIdentifier,
    foreground_pgrp: Option<ProcessIdentifier>,
) {
    daemon.controlling_terminal = ControllingTerminal::Assigned {
        session,
        foreground_pgrp,
    };
}

fn terminal_foreground_pgrp(daemon: &ProcessDaemon) -> Option<ProcessIdentifier> {
    match daemon.controlling_terminal {
        ControllingTerminal::Unassigned => None,
        ControllingTerminal::Assigned {
            foreground_pgrp, ..
        } => foreground_pgrp,
    }
}

fn lifecycle_message<const N: usize>(
    source: MessageSender,
    message_type: MessageType,
    payload: [u8; N],
) -> Message {
    let mut message_payload: [u8; Message::PAYLOAD_SIZE] = [0; Message::PAYLOAD_SIZE];
    message_payload[..N].copy_from_slice(&payload);
    Message::new(
        source,
        MessageReceiver::new(ProcessIdentifier::PROCD, ThreadIdentifier::NONE),
        message_type,
        None,
        message_payload,
    )
}

/// Builds the thread-termination event that the kernel publishes for `tid` of `pid`.
fn thread_termination_message(pid: ProcessIdentifier, tid: ThreadIdentifier) -> Message {
    let info: ThreadTerminationInfo = ThreadTerminationInfo::new(pid, tid, ExitStatus::ok());
    lifecycle_message(
        MessageSender::KERNEL,
        MessageType::ThreadTerminationEvent,
        info.to_ne_bytes(),
    )
}

/// Builds a wait for any child that `tid` of `waiter` blocked with `request_id`.
fn blocked_waiter(
    waiter: ProcessIdentifier,
    tid: ThreadIdentifier,
    request_id: RequestIdentifier,
) -> BlockedWaiter {
    BlockedWaiter {
        waiter,
        selector: WaitSelector::Any,
        response_context: ResponseContext::new(MessageSender::new(waiter, tid), request_id),
    }
}

/// Returns the thread identifier at `offset` from `base`.
fn thread_at(base: i32, offset: usize) -> ThreadIdentifier {
    let offset: i32 = i32::try_from(offset).expect("thread offset should fit in an i32");
    ThreadIdentifier::from(base + offset)
}

/// Builds a blocking wait for `child`.
fn wait_for_child(child: ProcessIdentifier) -> WaitMessage {
    WaitMessage::new(WaitTarget::Pid(child), 0)
}

/// Decodes the error code carried by an immediate wait reply.
fn wait_reply_error(reply: &Message) -> i32 {
    let system: SystemMessage =
        SystemMessage::from_bytes(reply.payload).expect("wait reply should be a system message");
    let process: ProcessManagementMessage = ProcessManagementMessage::from_bytes(system.payload)
        .expect("wait reply should be a process-management message");
    assert!(
        matches!(process.header, ProcessManagementMessageHeader::WaitResponse),
        "immediate reply should be a wait response"
    );
    WaitResponseMessage::from_bytes(process.payload).error
}

#[test]
fn normal_mode_handles_valid_process_creation_message() {
    let child: ProcessIdentifier = ProcessIdentifier::from(10);
    let info: ProcessCreationInfo =
        ProcessCreationInfo::new(child, ProcessIdentifier::KERNEL, ProcessRole::Daemon);
    let message: Message = lifecycle_message(
        MessageSender::KERNEL,
        MessageType::ProcessCreationEvent,
        info.to_ne_bytes(),
    );
    let mut daemon: ManuallyDrop<ProcessDaemon> = process_daemon(&[]);

    let status: Option<i32> = daemon.handle_message(message);

    assert_eq!(status, None, "process creation should not trigger shutdown");
    assert!(daemon.processes.contains_key(&child), "child should be registered");
}

#[test]
fn normal_mode_rejects_forged_process_creation_message() {
    let child: ProcessIdentifier = ProcessIdentifier::from(10);
    let info: ProcessCreationInfo =
        ProcessCreationInfo::new(child, ProcessIdentifier::KERNEL, ProcessRole::Daemon);
    let message: Message = lifecycle_message(
        MessageSender::new(ProcessIdentifier::from(20), ThreadIdentifier::from(21)),
        MessageType::ProcessCreationEvent,
        info.to_ne_bytes(),
    );
    let mut daemon: ManuallyDrop<ProcessDaemon> = process_daemon(&[]);

    let error: Error = parse_lifecycle_message(&message)
        .expect_err("non-kernel lifecycle message should be rejected");
    let status: Option<i32> = daemon.handle_message(message);

    assert_eq!(error.code, ErrorCode::PermissionDenied);
    assert_eq!(status, None, "forged creation should not trigger shutdown");
    assert!(daemon.processes.is_empty(), "forged creation should not change registry");
}

#[test]
fn normal_mode_handles_valid_process_termination_message() {
    let pid: ProcessIdentifier = ProcessIdentifier::from(10);
    let expected_status: i32 = 0x0102_0304;
    let info: ProcessTerminationInfo = ProcessTerminationInfo::new(
        pid,
        ExitStatus::from(expected_status as u32),
        ProcessIdentifier::KERNEL,
        ProcessRole::Init,
    );
    let message: Message = lifecycle_message(
        MessageSender::KERNEL,
        MessageType::ProcessTerminationEvent,
        info.to_ne_bytes(),
    );
    let mut daemon: ManuallyDrop<ProcessDaemon> =
        process_daemon(&[(pid, Some(process_identity(0)))]);
    daemon.blocked.push(BlockedWaiter {
        waiter: pid,
        selector: WaitSelector::Any,
        response_context: ResponseContext::new(
            MessageSender::new(pid, ThreadIdentifier::from(11)),
            RequestIdentifier::from_raw(1),
        ),
    });
    daemon.blocked.push(BlockedWaiter {
        waiter: pid,
        selector: WaitSelector::Any,
        response_context: ResponseContext::new(
            MessageSender::new(pid, ThreadIdentifier::from(12)),
            RequestIdentifier::from_raw(2),
        ),
    });

    let status: Option<i32> = daemon.handle_message(message);

    assert_eq!(status, Some(expected_status), "init termination should trigger shutdown");
    assert!(daemon.processes.is_empty(), "terminated init process should be removed");
    assert!(daemon.blocked.is_empty(), "all process waiters should be removed");
}

#[test]
fn normal_mode_rejects_forged_process_termination_message() {
    let pid: ProcessIdentifier = ProcessIdentifier::from(10);
    let info: ProcessTerminationInfo = ProcessTerminationInfo::new(
        pid,
        ExitStatus::ok(),
        ProcessIdentifier::KERNEL,
        ProcessRole::Init,
    );
    let message: Message = lifecycle_message(
        MessageSender::new(ProcessIdentifier::from(20), ThreadIdentifier::from(21)),
        MessageType::ProcessTerminationEvent,
        info.to_ne_bytes(),
    );
    let mut daemon: ManuallyDrop<ProcessDaemon> =
        process_daemon(&[(pid, Some(process_identity(0)))]);

    let status: Option<i32> = daemon.handle_message(message);

    assert_eq!(status, None, "forged termination should not trigger shutdown");
    assert!(daemon.processes.contains_key(&pid), "forged termination should be ignored");
}

#[test]
fn normal_mode_cleans_terminated_thread_waiters() {
    let pid: ProcessIdentifier = ProcessIdentifier::from(10);
    let other_pid: ProcessIdentifier = ProcessIdentifier::from(20);
    let terminated_tid: ThreadIdentifier = ThreadIdentifier::from(11);
    let sibling_tid: ThreadIdentifier = ThreadIdentifier::from(12);
    let info: ThreadTerminationInfo =
        ThreadTerminationInfo::new(pid, terminated_tid, ExitStatus::from(0x0102_0304_u32));
    let message: Message = lifecycle_message(
        MessageSender::KERNEL,
        MessageType::ThreadTerminationEvent,
        info.to_ne_bytes(),
    );
    let mut daemon: ManuallyDrop<ProcessDaemon> = process_daemon(&[
        (pid, Some(process_identity(0))),
        (other_pid, Some(process_identity(0))),
    ]);
    let first_terminated_context: ResponseContext = ResponseContext::new(
        MessageSender::new(pid, terminated_tid),
        RequestIdentifier::from_raw(1),
    );
    let sibling_context: ResponseContext =
        ResponseContext::new(MessageSender::new(pid, sibling_tid), RequestIdentifier::from_raw(2));
    let second_terminated_context: ResponseContext = ResponseContext::new(
        MessageSender::new(pid, terminated_tid),
        RequestIdentifier::from_raw(3),
    );
    let other_process_context: ResponseContext = ResponseContext::new(
        MessageSender::new(other_pid, terminated_tid),
        RequestIdentifier::from_raw(4),
    );
    daemon.blocked.push(BlockedWaiter {
        waiter: pid,
        selector: WaitSelector::Any,
        response_context: first_terminated_context,
    });
    daemon.blocked.push(BlockedWaiter {
        waiter: pid,
        selector: WaitSelector::Any,
        response_context: sibling_context,
    });
    daemon.blocked.push(BlockedWaiter {
        waiter: pid,
        selector: WaitSelector::Any,
        response_context: second_terminated_context,
    });
    daemon.blocked.push(BlockedWaiter {
        waiter: other_pid,
        selector: WaitSelector::Any,
        response_context: other_process_context,
    });

    let parsed: Option<LifecycleMessage> = parse_lifecycle_message(&message)
        .expect("kernel thread termination message should be decoded");
    let status: Option<i32> = daemon.handle_message(message);

    assert_eq!(parsed, Some(LifecycleMessage::ThreadTermination(info)));
    assert_eq!(status, None, "thread termination should not trigger shutdown");
    assert!(daemon.processes.contains_key(&pid), "thread event should not change registry");
    assert_eq!(daemon.blocked.len(), 2, "only terminated thread waits should be removed");
    assert_eq!(daemon.blocked[0].response_context, sibling_context);
    assert_eq!(daemon.blocked[1].response_context, other_process_context);

    let repeated_message: Message = lifecycle_message(
        MessageSender::KERNEL,
        MessageType::ThreadTerminationEvent,
        info.to_ne_bytes(),
    );
    let repeated_status: Option<i32> = daemon.handle_message(repeated_message);

    assert_eq!(repeated_status, None, "repeated thread termination should be harmless");
    assert_eq!(daemon.blocked.len(), 2, "repeated cleanup should leave other waits intact");
    assert_eq!(daemon.blocked[0].response_context, sibling_context);
    assert_eq!(daemon.blocked[1].response_context, other_process_context);
}

#[test]
fn normal_mode_rejects_forged_thread_termination_message() {
    let pid: ProcessIdentifier = ProcessIdentifier::from(10);
    let tid: ThreadIdentifier = ThreadIdentifier::from(11);
    let info: ThreadTerminationInfo = ThreadTerminationInfo::new(pid, tid, ExitStatus::ok());
    let message: Message = lifecycle_message(
        MessageSender::new(ProcessIdentifier::from(20), ThreadIdentifier::from(21)),
        MessageType::ThreadTerminationEvent,
        info.to_ne_bytes(),
    );
    let mut daemon: ManuallyDrop<ProcessDaemon> =
        process_daemon(&[(pid, Some(process_identity(0)))]);
    let context: ResponseContext =
        ResponseContext::new(MessageSender::new(pid, tid), RequestIdentifier::from_raw(1));
    daemon.blocked.push(BlockedWaiter {
        waiter: pid,
        selector: WaitSelector::Any,
        response_context: context,
    });

    let error: Error = parse_lifecycle_message(&message)
        .expect_err("non-kernel lifecycle message should be rejected");
    let status: Option<i32> = daemon.handle_message(message);

    assert_eq!(error.code, ErrorCode::PermissionDenied);
    assert_eq!(status, None, "forged thread termination should not trigger shutdown");
    assert!(daemon.processes.contains_key(&pid), "forged thread event should be ignored");
    assert_eq!(daemon.blocked.len(), 1, "forged thread event should not release blocked waits");
    assert_eq!(daemon.blocked[0].response_context, context);
}

#[test]
fn normal_mode_ignores_unexpected_message_types() {
    let pid: ProcessIdentifier = ProcessIdentifier::from(10);
    let tid: ThreadIdentifier = ThreadIdentifier::from(11);
    let mut daemon: ManuallyDrop<ProcessDaemon> =
        process_daemon(&[(pid, Some(process_identity(0)))]);
    let context: ResponseContext =
        ResponseContext::new(MessageSender::new(pid, tid), RequestIdentifier::from_raw(1));
    daemon.blocked.push(BlockedWaiter {
        waiter: pid,
        selector: WaitSelector::Any,
        response_context: context,
    });

    for message_type in [
        MessageType::Exception,
        MessageType::Interrupt,
        MessageType::Ikc,
    ] {
        let message: Message = lifecycle_message(
            MessageSender::new(ProcessIdentifier::from(20), ThreadIdentifier::from(21)),
            message_type,
            [],
        );

        let status: Option<i32> = daemon.handle_message(message);

        assert_eq!(status, None, "{message_type:?} should not trigger shutdown");
        assert!(daemon.processes.contains_key(&pid), "{message_type:?} should be ignored");
        assert_eq!(daemon.blocked.len(), 1, "{message_type:?} should retain blocked waits");
        assert_eq!(daemon.blocked[0].response_context, context);
    }
}

#[test]
fn shutdown_mode_ignores_valid_process_creation_message() {
    let pid: ProcessIdentifier = ProcessIdentifier::from(10);
    let child: ProcessIdentifier = ProcessIdentifier::from(11);
    let info: ProcessCreationInfo =
        ProcessCreationInfo::new(child, ProcessIdentifier::KERNEL, ProcessRole::Daemon);
    let message: Message = lifecycle_message(
        MessageSender::KERNEL,
        MessageType::ProcessCreationEvent,
        info.to_ne_bytes(),
    );
    let mut daemon: ManuallyDrop<ProcessDaemon> =
        process_daemon(&[(pid, Some(process_identity(0)))]);

    daemon.handle_shutdown_message(&message);

    assert_eq!(daemon.processes.len(), 1, "creation should not change registry");
    assert!(daemon.processes.contains_key(&pid), "tracked process should remain registered");
    assert!(!daemon.processes.contains_key(&child), "new process should not be registered");
}

#[test]
fn shutdown_mode_rejects_forged_process_creation_message() {
    let pid: ProcessIdentifier = ProcessIdentifier::from(10);
    let child: ProcessIdentifier = ProcessIdentifier::from(11);
    let info: ProcessCreationInfo =
        ProcessCreationInfo::new(child, ProcessIdentifier::KERNEL, ProcessRole::Daemon);
    let message: Message = lifecycle_message(
        MessageSender::new(ProcessIdentifier::from(20), ThreadIdentifier::from(21)),
        MessageType::ProcessCreationEvent,
        info.to_ne_bytes(),
    );
    let mut daemon: ManuallyDrop<ProcessDaemon> =
        process_daemon(&[(pid, Some(process_identity(0)))]);

    daemon.handle_shutdown_message(&message);

    assert_eq!(daemon.processes.len(), 1, "forged creation should not change registry");
    assert!(daemon.processes.contains_key(&pid), "tracked process should remain registered");
    assert!(!daemon.processes.contains_key(&child), "forged child should not be registered");
}

#[test]
fn shutdown_mode_handles_valid_process_termination_message() {
    let pid: ProcessIdentifier = ProcessIdentifier::from(10);
    let info: ProcessTerminationInfo = ProcessTerminationInfo::new(
        pid,
        ExitStatus::from(0x0102_0304_u32),
        ProcessIdentifier::KERNEL,
        ProcessRole::Daemon,
    );
    let message: Message = lifecycle_message(
        MessageSender::KERNEL,
        MessageType::ProcessTerminationEvent,
        info.to_ne_bytes(),
    );
    let mut daemon: ManuallyDrop<ProcessDaemon> =
        process_daemon(&[(pid, Some(process_identity(0)))]);

    daemon.handle_shutdown_message(&message);

    assert!(daemon.processes.is_empty(), "terminated daemon should be removed");
}

#[test]
fn shutdown_mode_rejects_forged_process_termination_message() {
    let pid: ProcessIdentifier = ProcessIdentifier::from(10);
    let info: ProcessTerminationInfo = ProcessTerminationInfo::new(
        pid,
        ExitStatus::ok(),
        ProcessIdentifier::KERNEL,
        ProcessRole::Daemon,
    );
    let message: Message = lifecycle_message(
        MessageSender::new(ProcessIdentifier::from(20), ThreadIdentifier::from(21)),
        MessageType::ProcessTerminationEvent,
        info.to_ne_bytes(),
    );
    let mut daemon: ManuallyDrop<ProcessDaemon> =
        process_daemon(&[(pid, Some(process_identity(0)))]);

    let error: Error = parse_lifecycle_message(&message)
        .expect_err("non-kernel lifecycle message should be rejected");
    daemon.handle_shutdown_message(&message);

    assert_eq!(error.code, ErrorCode::PermissionDenied);
    assert!(daemon.processes.contains_key(&pid), "forged termination should be ignored");
}

#[test]
fn shutdown_mode_cleans_terminated_thread_waiters() {
    let pid: ProcessIdentifier = ProcessIdentifier::from(10);
    let terminated_tid: ThreadIdentifier = ThreadIdentifier::from(11);
    let sibling_tid: ThreadIdentifier = ThreadIdentifier::from(12);
    let info: ThreadTerminationInfo =
        ThreadTerminationInfo::new(pid, terminated_tid, ExitStatus::from(0x0102_0304_u32));
    let message: Message = lifecycle_message(
        MessageSender::KERNEL,
        MessageType::ThreadTerminationEvent,
        info.to_ne_bytes(),
    );
    let mut daemon: ManuallyDrop<ProcessDaemon> =
        process_daemon(&[(pid, Some(process_identity(0)))]);
    let sibling_context: ResponseContext =
        ResponseContext::new(MessageSender::new(pid, sibling_tid), RequestIdentifier::from_raw(2));
    daemon.blocked.push(BlockedWaiter {
        waiter: pid,
        selector: WaitSelector::Any,
        response_context: ResponseContext::new(
            MessageSender::new(pid, terminated_tid),
            RequestIdentifier::from_raw(1),
        ),
    });
    daemon.blocked.push(BlockedWaiter {
        waiter: pid,
        selector: WaitSelector::Any,
        response_context: sibling_context,
    });

    daemon.handle_shutdown_message(&message);

    // Deregistering the owning process here would end the shutdown drain before the process
    // actually terminated.
    assert!(daemon.processes.contains_key(&pid), "thread event should not deregister process");
    assert_eq!(daemon.blocked.len(), 1, "terminated thread wait should be removed");
    assert_eq!(daemon.blocked[0].response_context, sibling_context);
}

#[test]
fn shutdown_mode_rejects_forged_thread_termination_message() {
    let pid: ProcessIdentifier = ProcessIdentifier::from(10);
    let tid: ThreadIdentifier = ThreadIdentifier::from(11);
    let info: ThreadTerminationInfo = ThreadTerminationInfo::new(pid, tid, ExitStatus::ok());
    let message: Message = lifecycle_message(
        MessageSender::new(ProcessIdentifier::from(20), ThreadIdentifier::from(21)),
        MessageType::ThreadTerminationEvent,
        info.to_ne_bytes(),
    );
    let mut daemon: ManuallyDrop<ProcessDaemon> =
        process_daemon(&[(pid, Some(process_identity(0)))]);
    let context: ResponseContext =
        ResponseContext::new(MessageSender::new(pid, tid), RequestIdentifier::from_raw(1));
    daemon.blocked.push(BlockedWaiter {
        waiter: pid,
        selector: WaitSelector::Any,
        response_context: context,
    });

    daemon.handle_shutdown_message(&message);

    assert!(daemon.processes.contains_key(&pid), "forged thread event should be ignored");
    assert_eq!(daemon.blocked.len(), 1, "forged thread event should retain blocked waits");
    assert_eq!(daemon.blocked[0].response_context, context);
}

#[test]
fn shutdown_mode_ignores_unrelated_message() {
    let pid: ProcessIdentifier = ProcessIdentifier::from(10);
    let message: Message = lifecycle_message(MessageSender::KERNEL, MessageType::Ipc, []);
    let mut daemon: ManuallyDrop<ProcessDaemon> =
        process_daemon(&[(pid, Some(process_identity(0)))]);

    let parsed: Option<LifecycleMessage> = parse_lifecycle_message(&message)
        .expect("unrelated message should not fail lifecycle parsing");
    daemon.handle_shutdown_message(&message);

    assert_eq!(parsed, None, "IPC message should not be treated as lifecycle traffic");
    assert!(daemon.processes.contains_key(&pid), "unrelated message should be ignored");
}

#[test]
fn relay_request_identifier_wraps_and_skips_active_identifiers() {
    let process: ProcessIdentifier = ProcessIdentifier::from(10);
    let mut daemon: ManuallyDrop<ProcessDaemon> =
        process_daemon(&[(process, Some(process_identity(1000)))]);
    let fork_request_id: RequestIdentifier = RequestIdentifier::from_raw(u32::MAX);
    let exec_request_id: RequestIdentifier = RequestIdentifier::from_raw(1);
    let setsid_request_id: RequestIdentifier = RequestIdentifier::from_raw(2);
    daemon
        .processes
        .get_mut(&process)
        .expect("process should exist")
        .fork_clone_request_id = Some(fork_request_id);
    daemon.pending_execs.push(PendingExec {
        process,
        request_id: exec_request_id,
        response_context: response_context(process),
    });
    daemon.pending_setsid.push(PendingSetSid {
        process,
        request_id: setsid_request_id,
        response_context: response_context(process),
    });
    daemon.next_request_id = u32::MAX;

    assert_eq!(daemon.allocate_request_id(), RequestIdentifier::from_raw(3));
    assert_eq!(daemon.next_request_id, 4);
}

#[test]
fn stale_exec_ack_keeps_matching_request_pending() {
    let process: ProcessIdentifier = ProcessIdentifier::from(10);
    let expected: RequestIdentifier = RequestIdentifier::from_raw(7);
    let mut daemon: ManuallyDrop<ProcessDaemon> =
        process_daemon(&[(process, Some(process_identity(1000)))]);
    daemon.pending_execs.push(PendingExec {
        process,
        request_id: expected,
        response_context: response_context(process),
    });

    daemon.handle_exec_ack(process, 0, RequestIdentifier::from_raw(8));

    assert_eq!(daemon.pending_execs.len(), 1);
    assert_eq!(daemon.pending_execs[0].request_id, expected);
}

#[test]
fn process_waiter_cleanup_removes_all_threads() {
    let process: ProcessIdentifier = ProcessIdentifier::from(10);
    let other_process: ProcessIdentifier = ProcessIdentifier::from(11);
    let other_context: ResponseContext = ResponseContext::new(
        MessageSender::new(other_process, ThreadIdentifier::from(20)),
        RequestIdentifier::from_raw(3),
    );
    let mut daemon: ManuallyDrop<ProcessDaemon> = process_daemon(&[
        (process, Some(process_identity(1000))),
        (other_process, Some(process_identity(1000))),
    ]);
    daemon.blocked.push(BlockedWaiter {
        waiter: process,
        selector: WaitSelector::Any,
        response_context: ResponseContext::new(
            MessageSender::new(process, ThreadIdentifier::from(20)),
            RequestIdentifier::from_raw(1),
        ),
    });
    daemon.blocked.push(BlockedWaiter {
        waiter: process,
        selector: WaitSelector::Any,
        response_context: ResponseContext::new(
            MessageSender::new(process, ThreadIdentifier::from(21)),
            RequestIdentifier::from_raw(2),
        ),
    });
    daemon.blocked.push(BlockedWaiter {
        waiter: other_process,
        selector: WaitSelector::Any,
        response_context: other_context,
    });

    daemon.cleanup_process_waiters(process);

    assert_eq!(daemon.blocked.len(), 1, "process cleanup should remove all process waits");
    assert_eq!(daemon.blocked[0].response_context, other_context);
}

#[test]
fn stale_fork_clone_ack_keeps_matching_request_pending() {
    let child: ProcessIdentifier = ProcessIdentifier::from(11);
    let expected: RequestIdentifier = RequestIdentifier::from_raw(7);
    let mut daemon: ManuallyDrop<ProcessDaemon> =
        process_daemon(&[(child, Some(process_identity(1000)))]);
    daemon
        .processes
        .get_mut(&child)
        .expect("child should exist")
        .fork_clone_request_id = Some(expected);

    daemon.handle_fork_clone_ack(child, 0, RequestIdentifier::from_raw(8));

    let record: &ProcessRecord = daemon.processes.get(&child).expect("child should exist");
    assert_eq!(record.fork_clone_request_id, Some(expected));
    assert!(!record.fork_clone_done);
    assert!(!record.fork_clone_failed);
}

#[test]
fn immediate_wait_response_stamps_request_id_and_exact_thread() {
    let process: ProcessIdentifier = ProcessIdentifier::from(10);
    let thread: ThreadIdentifier = ThreadIdentifier::from(20);
    let request_id: RequestIdentifier = RequestIdentifier::from_raw(0x12345678);
    let response_context: ResponseContext =
        ResponseContext::new(MessageSender::new(process, thread), request_id);
    let mut daemon: ManuallyDrop<ProcessDaemon> =
        process_daemon(&[(process, Some(process_identity(1000)))]);
    let mut response: Message = daemon
        .handle_wait(process, response_context, WaitMessage::new(WaitTarget::Any, 0))
        .expect("wait request should be handled")
        .expect("caller without children should receive an immediate response");

    response_context.prepare_response(&mut response);

    assert_eq!(
        { response.destination },
        MessageReceiver::new(process, thread),
        "response should target the requesting thread"
    );
    assert_eq!(
        RequestIdentifier::read_from(&response),
        request_id,
        "response should echo the request identifier"
    );
}

#[test]
fn blocked_waiter_retains_response_context() {
    let parent: ProcessIdentifier = ProcessIdentifier::from(10);
    let child: ProcessIdentifier = ProcessIdentifier::from(11);
    let thread: ThreadIdentifier = ThreadIdentifier::from(20);
    let request_id: RequestIdentifier = RequestIdentifier::from_raw(0x87654321);
    let response_context: ResponseContext =
        ResponseContext::new(MessageSender::new(parent, thread), request_id);
    let mut daemon: ManuallyDrop<ProcessDaemon> = process_daemon(&[
        (parent, Some(process_identity(1000))),
        (child, Some(process_identity(1000))),
    ]);
    daemon
        .processes
        .get_mut(&parent)
        .expect("parent record should exist")
        .children
        .push(child);

    let reply: Option<Message> = daemon
        .handle_wait(parent, response_context, WaitMessage::new(WaitTarget::Pid(child), 0))
        .expect("blocking wait should be accepted");

    assert!(reply.is_none(), "live child should block the waiter");
    assert_eq!(daemon.blocked.len(), 1, "waiter should be retained");
    let retained_context: ResponseContext = daemon.blocked[0].response_context;
    assert_eq!(
        retained_context, response_context,
        "waiter should retain the original response context"
    );

    let mut response: Message = message::wait_response(parent, child, 7, 0)
        .expect("deferred wait response should be constructed");
    retained_context.prepare_response(&mut response);
    assert_eq!(
        { response.destination },
        MessageReceiver::new(parent, thread),
        "deferred response should target the requesting thread"
    );
    assert_eq!(
        RequestIdentifier::read_from(&response),
        request_id,
        "deferred response should echo the request identifier"
    );
}

#[test]
fn concurrent_blocked_waiters_retain_distinct_response_contexts() {
    let parent: ProcessIdentifier = ProcessIdentifier::from(10);
    let child: ProcessIdentifier = ProcessIdentifier::from(11);
    let first_context: ResponseContext = ResponseContext::new(
        MessageSender::new(parent, ThreadIdentifier::from(20)),
        RequestIdentifier::from_raw(100),
    );
    let second_context: ResponseContext = ResponseContext::new(
        MessageSender::new(parent, ThreadIdentifier::from(21)),
        RequestIdentifier::from_raw(101),
    );
    let mut daemon: ManuallyDrop<ProcessDaemon> = process_daemon(&[
        (parent, Some(process_identity(1000))),
        (child, Some(process_identity(1000))),
    ]);
    daemon
        .processes
        .get_mut(&parent)
        .expect("parent record should exist")
        .children
        .push(child);

    for context in [first_context, second_context] {
        let reply: Option<Message> = daemon
            .handle_wait(parent, context, WaitMessage::new(WaitTarget::Pid(child), 0))
            .expect("blocking wait should be accepted");
        assert!(reply.is_none(), "live child should block the waiter");
    }

    assert_eq!(daemon.blocked.len(), 2, "both waits should remain queued");
    assert_eq!(daemon.blocked[0].response_context, first_context);
    assert_eq!(daemon.blocked[1].response_context, second_context);
}

#[test]
fn wait_cancel_removes_only_exact_request() {
    let parent: ProcessIdentifier = ProcessIdentifier::from(10);
    let first_tid: ThreadIdentifier = ThreadIdentifier::from(20);
    let second_tid: ThreadIdentifier = ThreadIdentifier::from(21);
    let first_id: RequestIdentifier = RequestIdentifier::from_raw(100);
    let second_id: RequestIdentifier = RequestIdentifier::from_raw(101);
    let mut daemon: ManuallyDrop<ProcessDaemon> =
        process_daemon(&[(parent, Some(process_identity(1000)))]);
    daemon.blocked.push(BlockedWaiter {
        waiter: parent,
        selector: WaitSelector::Any,
        response_context: ResponseContext::new(MessageSender::new(parent, first_tid), first_id),
    });
    daemon.blocked.push(BlockedWaiter {
        waiter: parent,
        selector: WaitSelector::Any,
        response_context: ResponseContext::new(MessageSender::new(parent, second_tid), second_id),
    });

    assert!(daemon.handle_wait_cancel(parent, first_tid, first_id));
    assert_eq!(daemon.blocked.len(), 1);
    assert_eq!(daemon.blocked[0].response_context.request_id, second_id);
}

#[test]
fn wait_cancellation_and_thread_termination_converge_in_either_order() {
    let parent: ProcessIdentifier = ProcessIdentifier::from(10);
    let terminated_tid: ThreadIdentifier = ThreadIdentifier::from(11);
    let sibling_tid: ThreadIdentifier = ThreadIdentifier::from(12);
    let request_id: RequestIdentifier = RequestIdentifier::from_raw(7);

    for cancel_first in [true, false] {
        let mut daemon: ManuallyDrop<ProcessDaemon> =
            process_daemon(&[(parent, Some(process_identity(1000)))]);
        daemon
            .blocked
            .push(blocked_waiter(parent, terminated_tid, request_id));
        // The sibling reuses the request identifier so that only the thread disambiguates them.
        let sibling: BlockedWaiter = blocked_waiter(parent, sibling_tid, request_id);
        let sibling_context: ResponseContext = sibling.response_context;
        daemon.blocked.push(sibling);

        let cancelled: bool = if cancel_first {
            let cancelled: bool = daemon.handle_wait_cancel(parent, terminated_tid, request_id);
            let status: Option<i32> =
                daemon.handle_message(thread_termination_message(parent, terminated_tid));
            assert_eq!(status, None, "thread termination should not trigger shutdown");
            cancelled
        } else {
            let status: Option<i32> =
                daemon.handle_message(thread_termination_message(parent, terminated_tid));
            assert_eq!(status, None, "thread termination should not trigger shutdown");
            daemon.handle_wait_cancel(parent, terminated_tid, request_id)
        };

        assert_eq!(
            cancelled, cancel_first,
            "cancellation should win only when it precedes thread termination"
        );
        assert_eq!(daemon.blocked.len(), 1, "both orders should remove exactly one wait");
        assert_eq!(daemon.blocked[0].response_context, sibling_context);
    }
}

#[test]
fn terminated_wait_threads_do_not_exhaust_waiter_capacity() {
    const FIRST_CYCLE_TID: i32 = 100;
    const FIRST_LIVE_TID: i32 = 1000;
    let parent: ProcessIdentifier = ProcessIdentifier::from(10);
    let child: ProcessIdentifier = ProcessIdentifier::from(11);
    let mut daemon: ManuallyDrop<ProcessDaemon> = process_daemon(&[
        (parent, Some(process_identity(1000))),
        (child, Some(process_identity(1000))),
    ]);
    daemon
        .processes
        .get_mut(&parent)
        .expect("parent record should exist")
        .children
        .push(child);

    // Each cycle blocks one thread on the live child and then terminates that thread, as a thread
    // interrupted in waitpid() does when it exits before the child terminates.
    for cycle in 0..(2 * MAX_BLOCKED_WAITERS) {
        let tid: ThreadIdentifier = thread_at(FIRST_CYCLE_TID, cycle);
        let context: ResponseContext =
            ResponseContext::new(MessageSender::new(parent, tid), RequestIdentifier::from_raw(1));
        let reply: Option<Message> = daemon
            .handle_wait(parent, context, wait_for_child(child))
            .expect("blocking wait should be accepted");
        assert!(reply.is_none(), "live child should block the wait in cycle {cycle}");
        let status: Option<i32> = daemon.handle_message(thread_termination_message(parent, tid));
        assert_eq!(status, None, "thread termination should not trigger shutdown");
        assert!(daemon.blocked.is_empty(), "terminated thread left its wait in cycle {cycle}");
    }

    // Terminated threads left no residue, so live threads can still fill every waiter slot.
    for slot in 0..MAX_BLOCKED_WAITERS {
        let context: ResponseContext = ResponseContext::new(
            MessageSender::new(parent, thread_at(FIRST_LIVE_TID, slot)),
            RequestIdentifier::from_raw(1),
        );
        let reply: Option<Message> = daemon
            .handle_wait(parent, context, wait_for_child(child))
            .expect("blocking wait should be accepted");
        assert!(reply.is_none(), "waiter slot {slot} should be available");
    }
    let overflow_context: ResponseContext = ResponseContext::new(
        MessageSender::new(parent, thread_at(FIRST_LIVE_TID, MAX_BLOCKED_WAITERS)),
        RequestIdentifier::from_raw(1),
    );
    let overflow: Message = daemon
        .handle_wait(parent, overflow_context, wait_for_child(child))
        .expect("overflowing wait should be handled")
        .expect("overflowing wait should be answered immediately");
    assert_eq!(wait_reply_error(&overflow), ErrorCode::NoBufferSpace.get());
}

#[test]
fn process_exit_after_thread_terminations_cleans_every_waiting_thread() {
    let exiting: ProcessIdentifier = ProcessIdentifier::from(10);
    let survivor: ProcessIdentifier = ProcessIdentifier::from(20);
    let tids: [ThreadIdentifier; 3] = [
        ThreadIdentifier::from(11),
        ThreadIdentifier::from(12),
        ThreadIdentifier::from(13),
    ];
    let mut daemon: ManuallyDrop<ProcessDaemon> = process_daemon(&[
        (exiting, Some(process_identity(1000))),
        (survivor, Some(process_identity(1000))),
    ]);
    for (index, tid) in tids.iter().enumerate() {
        let request_id: u32 = u32::try_from(index + 1).expect("request identifier should fit");
        daemon
            .blocked
            .push(blocked_waiter(exiting, *tid, RequestIdentifier::from_raw(request_id)));
    }
    // A nested signal handler may block a second wait on the same thread.
    daemon
        .blocked
        .push(blocked_waiter(exiting, tids[0], RequestIdentifier::from_raw(99)));
    let survivor_waiter: BlockedWaiter =
        blocked_waiter(survivor, ThreadIdentifier::from(21), RequestIdentifier::from_raw(1));
    let survivor_context: ResponseContext = survivor_waiter.response_context;
    daemon.blocked.push(survivor_waiter);

    // On a whole-process exit the kernel publishes every thread termination before the process
    // termination, so each thread event must release that thread's waits on its own.
    for tid in tids {
        let status: Option<i32> = daemon.handle_message(thread_termination_message(exiting, tid));
        assert_eq!(status, None, "thread termination should not trigger shutdown");
        assert!(
            !daemon
                .blocked
                .iter()
                .any(|waiter| waiter.waiter == exiting
                    && waiter.response_context.receiver.tid == tid),
            "thread termination left a wait owned by the terminated thread"
        );
    }
    assert_eq!(daemon.blocked.len(), 1, "only the surviving process should still be waiting");

    let info: ProcessTerminationInfo = ProcessTerminationInfo::new(
        exiting,
        ExitStatus::ok(),
        ProcessIdentifier::KERNEL,
        ProcessRole::User,
    );
    let status: Option<i32> = daemon.handle_message(lifecycle_message(
        MessageSender::KERNEL,
        MessageType::ProcessTerminationEvent,
        info.to_ne_bytes(),
    ));

    assert_eq!(status, None, "user process termination should not trigger shutdown");
    assert!(!daemon.processes.contains_key(&exiting), "unreapable process should be dropped");
    assert_eq!(daemon.blocked.len(), 1, "process termination should keep unrelated waits");
    assert_eq!(daemon.blocked[0].response_context, survivor_context);
}

#[test]
fn authorize_kill_allows_root_caller() {
    let caller: ProcessIdentifier = ProcessIdentifier::from(10);
    let target: ProcessIdentifier = ProcessIdentifier::from(11);
    let daemon: ManuallyDrop<ProcessDaemon> = process_daemon(&[
        (caller, Some(process_identity(0))),
        (target, Some(process_identity(1000))),
    ]);

    assert!(daemon.authorize_kill(caller, target).is_ok());
}

#[test]
fn authorize_kill_allows_same_user() {
    let caller: ProcessIdentifier = ProcessIdentifier::from(10);
    let target: ProcessIdentifier = ProcessIdentifier::from(11);
    let daemon: ManuallyDrop<ProcessDaemon> = process_daemon(&[
        (caller, Some(process_identity(1000))),
        (target, Some(process_identity(1000))),
    ]);

    assert!(daemon.authorize_kill(caller, target).is_ok());
}

#[test]
fn authorize_kill_rejects_different_user() {
    let caller: ProcessIdentifier = ProcessIdentifier::from(10);
    let target: ProcessIdentifier = ProcessIdentifier::from(11);
    let daemon: ManuallyDrop<ProcessDaemon> = process_daemon(&[
        (caller, Some(process_identity(1000))),
        (target, Some(process_identity(1001))),
    ]);
    let error: Error = daemon
        .authorize_kill(caller, target)
        .expect_err("different users should not be able to signal each other");

    assert_eq!(error.code, ErrorCode::PermissionDenied);
}

#[test]
fn authorize_kill_rejects_unknown_caller() {
    let caller: ProcessIdentifier = ProcessIdentifier::from(10);
    let target: ProcessIdentifier = ProcessIdentifier::from(11);
    let daemon: ManuallyDrop<ProcessDaemon> =
        process_daemon(&[(target, Some(process_identity(1000)))]);
    let error: Error = daemon
        .authorize_kill(caller, target)
        .expect_err("unknown caller should not be authorized");

    assert_eq!(error.code, ErrorCode::NoSuchProcess);
}

#[test]
fn authorize_kill_rejects_unknown_target() {
    let caller: ProcessIdentifier = ProcessIdentifier::from(10);
    let target: ProcessIdentifier = ProcessIdentifier::from(11);
    let daemon: ManuallyDrop<ProcessDaemon> =
        process_daemon(&[(caller, Some(process_identity(1000)))]);
    let error: Error = daemon
        .authorize_kill(caller, target)
        .expect_err("unknown target should not be authorized");

    assert_eq!(error.code, ErrorCode::NoSuchProcess);
}

#[test]
fn authorize_kill_rejects_missing_caller_identity() {
    let caller: ProcessIdentifier = ProcessIdentifier::from(10);
    let target: ProcessIdentifier = ProcessIdentifier::from(11);
    let daemon: ManuallyDrop<ProcessDaemon> =
        process_daemon(&[(caller, None), (target, Some(process_identity(1000)))]);
    let error: Error = daemon
        .authorize_kill(caller, target)
        .expect_err("caller without identity should not be authorized");

    assert_eq!(error.code, ErrorCode::NoSuchProcess);
}

#[test]
fn authorize_kill_rejects_missing_target_identity() {
    let caller: ProcessIdentifier = ProcessIdentifier::from(10);
    let target: ProcessIdentifier = ProcessIdentifier::from(11);
    let daemon: ManuallyDrop<ProcessDaemon> =
        process_daemon(&[(caller, Some(process_identity(1000))), (target, None)]);
    let error: Error = daemon
        .authorize_kill(caller, target)
        .expect_err("target without identity should not be authorized");

    assert_eq!(error.code, ErrorCode::NoSuchProcess);
}

#[test]
fn terminal_session_detaches_only_after_acknowledgement() {
    let leader: ProcessIdentifier = ProcessIdentifier::from(10);
    let foreground: ProcessIdentifier = ProcessIdentifier::from(11);
    let detached: ProcessIdentifier = ProcessIdentifier::from(12);
    let mut daemon: ManuallyDrop<ProcessDaemon> = process_daemon(&[
        (leader, Some(process_identity(1000))),
        (foreground, Some(process_identity(1000))),
        (detached, Some(process_identity(1000))),
    ]);
    for pid in [foreground, detached] {
        let record: &mut ProcessRecord = daemon.processes.get_mut(&pid).expect("process");
        record.sid = leader;
        record.pgid = foreground;
    }
    assign_controlling_terminal(&mut daemon, leader, Some(foreground));

    assert!(daemon.process_has_controlling_terminal(detached));
    daemon
        .begin_setsid(detached, response_context(detached))
        .expect("begin setsid");
    assert!(daemon.process_has_controlling_terminal(detached));
    assert_eq!(daemon.processes.get(&detached).expect("process").sid, leader);

    let request_id: RequestIdentifier = daemon.pending_setsid[0].request_id;
    let response: Option<(ResponseContext, Message)> = daemon.handle_terminal_detach_ack(
        detached,
        TerminalDetachAckMessage::STATUS_SUCCESS,
        request_id,
    );

    assert!(response.is_some());
    assert!(daemon.pending_setsid.is_empty());
    assert_eq!(daemon.processes.get(&detached).expect("process").sid, detached);
    assert!(!daemon.process_has_controlling_terminal(detached));
}

#[test]
fn setpgid_rejects_process_with_pending_setsid() {
    let parent: ProcessIdentifier = ProcessIdentifier::from(10);
    let process: ProcessIdentifier = ProcessIdentifier::from(11);
    let mut daemon: ManuallyDrop<ProcessDaemon> = process_daemon(&[
        (parent, Some(process_identity(1000))),
        (process, Some(process_identity(1000))),
    ]);
    let record: &mut ProcessRecord = daemon.processes.get_mut(&process).expect("process");
    record.parent = Some(parent);
    record.sid = parent;
    record.pgid = parent;
    daemon
        .begin_setsid(process, response_context(process))
        .expect("begin setsid");

    let error: Error = daemon
        .job_control_setpgid(parent, process, process)
        .expect_err("setpgid should not race a pending setsid");

    assert_eq!(error.code, ErrorCode::ResourceBusy);
    assert_eq!(daemon.processes.get(&process).expect("process").pgid, parent);
}

#[test]
fn terminal_detach_failure_preserves_session() {
    let leader: ProcessIdentifier = ProcessIdentifier::from(10);
    let process: ProcessIdentifier = ProcessIdentifier::from(11);
    let mut daemon: ManuallyDrop<ProcessDaemon> = process_daemon(&[
        (leader, Some(process_identity(1000))),
        (process, Some(process_identity(1000))),
    ]);
    let record: &mut ProcessRecord = daemon.processes.get_mut(&process).expect("process");
    record.sid = leader;
    record.pgid = leader;
    assign_controlling_terminal(&mut daemon, leader, Some(leader));
    daemon
        .begin_setsid(process, response_context(process))
        .expect("begin setsid");
    let request_id: RequestIdentifier = daemon.pending_setsid[0].request_id;

    let response: Option<(ResponseContext, Message)> =
        daemon.handle_terminal_detach_ack(process, ErrorCode::TryAgain.get(), request_id);

    assert!(response.is_some());
    assert!(daemon.pending_setsid.is_empty());
    let record: &ProcessRecord = daemon.processes.get(&process).expect("process");
    assert_eq!(record.sid, leader);
    assert_eq!(record.pgid, leader);
}

#[test]
fn stale_terminal_detach_ack_keeps_request_pending() {
    let leader: ProcessIdentifier = ProcessIdentifier::from(10);
    let process: ProcessIdentifier = ProcessIdentifier::from(11);
    let expected: RequestIdentifier = RequestIdentifier::from_raw(7);
    let mut daemon: ManuallyDrop<ProcessDaemon> = process_daemon(&[
        (leader, Some(process_identity(1000))),
        (process, Some(process_identity(1000))),
    ]);
    daemon.pending_setsid.push(PendingSetSid {
        process,
        request_id: expected,
        response_context: response_context(process),
    });

    let response: Option<(ResponseContext, Message)> = daemon.handle_terminal_detach_ack(
        process,
        TerminalDetachAckMessage::STATUS_SUCCESS,
        RequestIdentifier::from_raw(8),
    );

    assert!(response.is_none());
    assert_eq!(daemon.pending_setsid.len(), 1);
    assert_eq!(daemon.pending_setsid[0].request_id, expected);
    assert_eq!(daemon.processes.get(&process).expect("process").sid, process);
}

#[test]
fn terminal_operations_reject_detached_session() {
    let owner: ProcessIdentifier = ProcessIdentifier::from(10);
    let detached: ProcessIdentifier = ProcessIdentifier::from(11);
    let mut daemon: ManuallyDrop<ProcessDaemon> = process_daemon(&[
        (owner, Some(process_identity(1000))),
        (detached, Some(process_identity(1000))),
    ]);
    assign_controlling_terminal(&mut daemon, owner, Some(owner));

    let get_error: Error = daemon
        .job_control_tcgetpgrp(detached)
        .expect_err("detached session should not query foreground group");
    let set_error: Error = daemon
        .job_control_tcsetpgrp(detached, detached)
        .expect_err("detached session should not set foreground group");

    assert_eq!(get_error.code, ErrorCode::NotTerminal);
    assert_eq!(set_error.code, ErrorCode::NotTerminal);
}

#[test]
fn terminal_access_allows_vfsd_to_report_any_process() {
    let reader: ProcessIdentifier = ProcessIdentifier::from(10);
    let notification: TerminalAccessMessage = TerminalAccessMessage::new(reader, false);

    assert_eq!(
        ProcessDaemon::terminal_access_subject(ProcessIdentifier::VFSD, &notification),
        Some(reader)
    );
}

#[test]
fn terminal_access_allows_self_report() {
    let writer: ProcessIdentifier = ProcessIdentifier::from(10);
    let notification: TerminalAccessMessage = TerminalAccessMessage::new(writer, true);

    assert_eq!(ProcessDaemon::terminal_access_subject(writer, &notification), Some(writer));
}

#[test]
fn terminal_access_rejects_cross_process_report() {
    let reporter: ProcessIdentifier = ProcessIdentifier::from(10);
    let target: ProcessIdentifier = ProcessIdentifier::from(11);
    let notification: TerminalAccessMessage = TerminalAccessMessage::new(target, true);

    assert_eq!(ProcessDaemon::terminal_access_subject(reporter, &notification), None);
}

#[test]
fn foreground_pgrp_clears_when_last_member_is_removed() {
    let leader: ProcessIdentifier = ProcessIdentifier::from(10);
    let mut daemon: ManuallyDrop<ProcessDaemon> =
        process_daemon(&[(leader, Some(process_identity(0)))]);
    assign_controlling_terminal(&mut daemon, leader, Some(leader));

    daemon.processes.remove(&leader);
    daemon.clear_foreground_pgrp_if_empty();

    assert_eq!(terminal_foreground_pgrp(&daemon), None);
}

#[test]
fn foreground_pgrp_ignores_zombie_members() {
    let leader: ProcessIdentifier = ProcessIdentifier::from(10);
    let member: ProcessIdentifier = ProcessIdentifier::from(11);
    let mut daemon: ManuallyDrop<ProcessDaemon> = process_daemon(&[
        (leader, Some(process_identity(0))),
        (member, Some(process_identity(0))),
    ]);
    assign_controlling_terminal(&mut daemon, leader, Some(leader));
    daemon
        .processes
        .get_mut(&member)
        .expect("member record")
        .pgid = leader;

    daemon
        .processes
        .get_mut(&leader)
        .expect("leader record")
        .zombie = Some(0);
    daemon
        .processes
        .get_mut(&member)
        .expect("member record")
        .zombie = Some(0);
    daemon.clear_foreground_pgrp_if_empty();

    assert_eq!(terminal_foreground_pgrp(&daemon), None);
}

#[test]
fn foreground_pgrp_stays_when_group_has_live_member() {
    let leader: ProcessIdentifier = ProcessIdentifier::from(10);
    let member: ProcessIdentifier = ProcessIdentifier::from(11);
    let mut daemon: ManuallyDrop<ProcessDaemon> = process_daemon(&[
        (leader, Some(process_identity(0))),
        (member, Some(process_identity(0))),
    ]);
    assign_controlling_terminal(&mut daemon, leader, Some(leader));
    daemon
        .processes
        .get_mut(&member)
        .expect("member record")
        .pgid = leader;

    daemon
        .processes
        .get_mut(&leader)
        .expect("leader record")
        .zombie = Some(0);
    daemon.clear_foreground_pgrp_if_empty();

    assert_eq!(terminal_foreground_pgrp(&daemon), Some(leader));
}
