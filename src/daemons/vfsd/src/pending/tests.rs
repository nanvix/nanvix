// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::hostfs_api::{
    set_kind,
    StatResponse,
};
use ::sys::ipc::{
    MessageSender,
    RequestIdentifier,
};
use ::syscall::SystemCallMessageKind;
use ::vfs::path::vfs_resolve_path;

fn pending(kind: PendingOpKind) -> PendingOp {
    let process = ProcessIdentifier::from(10);
    let thread = ThreadIdentifier::from(20);
    PendingOp {
        response_context: ResponseContext::new(
            MessageSender::new(process, thread),
            RequestIdentifier::from_raw(1),
        ),
        source_tid: thread,
        source_pid: process,
        kind,
    }
}

fn stat_payload(status: i32) -> [u8; Message::PAYLOAD_SIZE] {
    let mut payload = [0; Message::PAYLOAD_SIZE];
    set_kind(&mut payload, SystemCallMessageKind::HostFsStatResponse as u16);
    StatResponse {
        status,
        size: 42,
        mode: 0o644,
        is_dir: 0,
    }
    .encode(&mut payload);
    payload
}

fn lstat_payload(kind: SystemCallMessageKind) -> [u8; Message::PAYLOAD_SIZE] {
    let mut payload = [0; Message::PAYLOAD_SIZE];
    set_kind(&mut payload, kind as u16);
    LstatResponse {
        status: 0,
        size: 42,
        mode: 0o644,
        kind: file_kind::REGULAR,
    }
    .encode(&mut payload);
    payload
}

#[test]
fn bad_fd_protocol_error_maps_to_bad_file() {
    assert_eq!(hostfs_error_to_code(::hostfs_api::HOSTFS_ERR_BAD_FD), ErrorCode::BadFile);
}

#[test]
fn successful_stat_waits_for_timestamps() {
    let mut op = pending(PendingOpKind::Stat);
    let payload = stat_payload(0);

    assert!(matches!(stage_stat_metadata(&mut op, &payload), StatMetadataStep::Wait));
    assert!(matches!(op.kind, PendingOpKind::StatTimes { .. }));
}

#[test]
fn failed_stat_completes_without_timestamps() {
    let mut op = pending(PendingOpKind::Stat);
    let payload = stat_payload(::hostfs_api::HOSTFS_ERR_NOT_FOUND);

    assert!(matches!(stage_stat_metadata(&mut op, &payload), StatMetadataStep::Complete));
    assert!(matches!(op.kind, PendingOpKind::Stat));
}

#[test]
fn path_stat_variants_wait_for_timestamps() {
    let mut lstat = pending(PendingOpKind::Lstat);
    let payload = lstat_payload(SystemCallMessageKind::HostFsLstatResponse);
    assert!(matches!(stage_stat_metadata(&mut lstat, &payload), StatMetadataStep::Wait));
    assert!(matches!(lstat.kind, PendingOpKind::LstatTimes { .. }));

    let mut pathstat = pending(PendingOpKind::PathStat);
    let payload = lstat_payload(SystemCallMessageKind::HostFsPathStatResponse);
    assert!(matches!(stage_stat_metadata(&mut pathstat, &payload), StatMetadataStep::Wait));
    assert!(matches!(pathstat.kind, PendingOpKind::PathStatTimes { .. }));
}

#[test]
fn chdir_retains_path_while_waiting_for_timestamps() {
    // No filesystem lookup happens here. The guest path stays `/mnt/dir/../child//`, while
    // stripping the mount prefix produces `dir/../child//` for the host. If `dir` is an
    // ordinary directory this reaches `/mnt/child`; a symlink may select a different parent.
    // Both spellings must survive the pending metadata/timestamp round trip unchanged.
    let guest = vfs_resolve_path(0, "/mnt/dir/../child//").expect("anchor hostfs directory");
    let path = RoutedHostPath::new(guest).expect("route hostfs directory");
    assert_eq!(path.host_path().as_str(), "dir/../child//");
    assert_eq!(path.clone().into_guest_path().as_str(), "/mnt/dir/../child//");
    let mut op = pending(PendingOpKind::Chdir { path: path.clone() });
    let payload = lstat_payload(SystemCallMessageKind::HostFsPathStatResponse);

    assert!(matches!(stage_stat_metadata(&mut op, &payload), StatMetadataStep::Wait));
    match op.kind {
        PendingOpKind::ChdirTimes { path: actual, .. } => assert_eq!(actual, path),
        _ => panic!("chdir should wait for timestamps"),
    }
}

#[test]
fn purged_staged_stat_is_discarded_on_metadata() {
    let mut queue = PendingQueue::new();
    let op_id = OperationId::from_raw(7);
    let op = pending(PendingOpKind::Stat);
    let process = op.source_pid;
    queue.insert(op_id, op).unwrap();
    let payload = stat_payload(0);
    let step = stage_stat_metadata(queue.get_mut(op_id).unwrap(), &payload);
    assert!(matches!(step, StatMetadataStep::Wait));

    queue.purge_pid(process);

    assert!(queue.discard_abandoned_operation(op_id));
    assert!(!queue.discard_abandoned_operation(op_id));
}
