// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn killpg_target_pid_maps_zero_to_caller() {
    assert_eq!(target_pid_for_group(0, 7), 7);
    assert_eq!(target_pid_for_group(3, 7), 3);
}

#[test]
fn killpg_rejects_negative_pgrp() {
    // Process groups are not supported, so negative pgrp selectors must be rejected outright
    // rather than forwarded to kill().
    assert_eq!(unsafe { killpg(-1, 2) }, -1);
    assert_eq!(unsafe { killpg(-2, 2) }, -1);
}

#[test]
fn killpg_reports_failure_on_host() {
    // The host build cannot deliver signals, so killpg() must fail rather than reference any
    // guest-only symbol. This guards against reintroducing an unconditional FFI call that
    // would break the host build.
    assert_eq!(unsafe { killpg(1, 2) }, -1);
}
