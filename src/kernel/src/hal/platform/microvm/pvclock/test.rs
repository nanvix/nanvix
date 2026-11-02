// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::reconcile_snapshot_time;

/// Verifies that a lagging host sample cannot move the guest timeline backwards.
fn test_reconcile_snapshot_time_clamps_lagging_host_sample() -> bool {
    let host_time_ns: u64 = 110_300_000;
    let previous_base_ns: u64 = 100_800_000;
    let previous_base_ticks: u32 = 100;
    let snapshot_ticks: u32 = 110;
    let expected_time_ns: u64 = 110_800_000;

    let actual_time_ns: u64 = reconcile_snapshot_time(
        host_time_ns,
        previous_base_ns,
        previous_base_ticks,
        snapshot_ticks,
    );
    if actual_time_ns != expected_time_ns {
        error!(
            "lagging host sample was not clamped (expected={expected_time_ns}, \
             actual={actual_time_ns})"
        );
        return false;
    }

    true
}

/// Verifies that a host sample ahead of guest interpolation is adopted.
fn test_reconcile_snapshot_time_adopts_leading_host_sample() -> bool {
    let host_time_ns: u64 = 111_200_000;
    let previous_base_ns: u64 = 100_800_000;
    let previous_base_ticks: u32 = 100;
    let snapshot_ticks: u32 = 110;

    let actual_time_ns: u64 = reconcile_snapshot_time(
        host_time_ns,
        previous_base_ns,
        previous_base_ticks,
        snapshot_ticks,
    );
    if actual_time_ns != host_time_ns {
        error!(
            "leading host sample was not adopted (expected={host_time_ns}, \
             actual={actual_time_ns})"
        );
        return false;
    }

    true
}

/// Runs all pvclock in-kernel tests.
pub(super) fn test() -> bool {
    let mut passed: bool = true;
    passed &= run_test!(test_reconcile_snapshot_time_clamps_lagging_host_sample);
    passed &= run_test!(test_reconcile_snapshot_time_adopts_leading_host_sample);
    passed
}
