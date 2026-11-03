// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn classify_daemon_when_flagged() {
    // A daemon is classified as such regardless of its parent or identifier.
    assert_eq!(ProcessRole::classify(ProcessIdentifier::KERNEL, true), ProcessRole::Daemon);
    assert_eq!(ProcessRole::classify(ProcessIdentifier::from(4), true), ProcessRole::Daemon);
}

#[test]
fn classify_init_by_kernel_lineage() {
    assert_eq!(ProcessRole::classify(ProcessIdentifier::KERNEL, false), ProcessRole::Init);
}

#[test]
fn classify_user_by_non_kernel_parent() {
    let parent: ProcessIdentifier = ProcessIdentifier::from(4);
    assert_eq!(ProcessRole::classify(parent, false), ProcessRole::User);
}

#[test]
fn termination_info_round_trip() {
    let info: ProcessTerminationInfo = ProcessTerminationInfo::new(
        ProcessIdentifier::from(5),
        ExitStatus::ok(),
        ProcessIdentifier::from(4),
        ProcessRole::User,
    );
    assert_eq!(ProcessTerminationInfo::from_ne_bytes(info.to_ne_bytes()), info);
}

#[test]
fn termination_info_round_trip_preserves_kernel_spawned_roles() {
    // procd routes a termination on the role carried in this event: an `Init` termination
    // triggers system shutdown and a `Daemon` termination triggers deregistration (and a crash
    // shutdown on a non-zero status). The role must therefore survive the native-endian round
    // trip intact. If a kernel-spawned process's role decoded as `User`, procd would silently
    // reap it as an ordinary forked child and never bring the system down — the regression
    // tracked in issue #2541.
    for role in [ProcessRole::Init, ProcessRole::Daemon] {
        let info: ProcessTerminationInfo = ProcessTerminationInfo::new(
            ProcessIdentifier::from(4),
            ExitStatus::ok(),
            ProcessIdentifier::KERNEL,
            role,
        );
        assert_eq!(ProcessTerminationInfo::from_ne_bytes(info.to_ne_bytes()), info);
    }
}

#[test]
fn process_role_round_trips_through_raw_u32() {
    // The kernel encodes the role as a raw `u32` and procd decodes it; both sides must agree on
    // each value so the role that drives the shutdown / deregistration / reap decision is the
    // one the kernel assigned.
    assert_eq!(ProcessRole::Init.to_u32(), 0);
    assert_eq!(ProcessRole::Daemon.to_u32(), 1);
    assert_eq!(ProcessRole::User.to_u32(), 2);
    assert_eq!(ProcessRole::from_u32(ProcessRole::Init.to_u32()), ProcessRole::Init);
    assert_eq!(ProcessRole::from_u32(ProcessRole::Daemon.to_u32()), ProcessRole::Daemon);
    assert_eq!(ProcessRole::from_u32(ProcessRole::User.to_u32()), ProcessRole::User);
}

#[test]
fn process_role_unknown_value_decodes_to_user() {
    // An unrecognized raw value decodes to `User` — the only role that triggers no system-wide
    // action — so a malformed event cannot spuriously shut the system down.
    assert_eq!(ProcessRole::from_u32(3), ProcessRole::User);
    assert_eq!(ProcessRole::from_u32(u32::MAX), ProcessRole::User);
}
