// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn creation_info_round_trip() {
    let info: ProcessCreationInfo = ProcessCreationInfo::new(
        ProcessIdentifier::from(5),
        ProcessIdentifier::from(4),
        ProcessRole::Init,
    );
    assert_eq!(ProcessCreationInfo::from_ne_bytes(info.to_ne_bytes()), info);
}

#[test]
fn creation_info_round_trip_preserves_roles() {
    for role in [ProcessRole::Init, ProcessRole::Daemon, ProcessRole::User] {
        let info: ProcessCreationInfo =
            ProcessCreationInfo::new(ProcessIdentifier::from(5), ProcessIdentifier::KERNEL, role);
        assert_eq!(ProcessCreationInfo::from_ne_bytes(info.to_ne_bytes()), info);
    }
}
