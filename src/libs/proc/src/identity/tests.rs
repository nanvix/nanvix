// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::ProcessIdentity;
use ::sys::pm::{
    GroupIdentifier,
    UserIdentifier,
};

fn identity(uid: usize) -> ProcessIdentity {
    ProcessIdentity::new(UserIdentifier::from(uid), GroupIdentifier::ROOT)
}

#[test]
fn can_signal_allows_same_real_uid() {
    let caller: ProcessIdentity = identity(1000);
    let target: ProcessIdentity = identity(1000);

    assert!(caller.can_signal(&target));
}

#[test]
fn can_signal_allows_root() {
    let caller: ProcessIdentity = identity(0);
    let target: ProcessIdentity = identity(1000);

    assert!(caller.can_signal(&target));
}

#[test]
fn can_signal_rejects_different_uid() {
    let caller: ProcessIdentity = identity(1000);
    let target: ProcessIdentity = identity(1001);

    assert!(!caller.can_signal(&target));
}
