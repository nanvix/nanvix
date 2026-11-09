// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::std::thread;

#[test]
fn start_waits_for_initial_wait() {
    let timeout: BackoffTimeout =
        BackoffTimeout::start(Duration::from_millis(10), Duration::from_secs(1));
    assert_eq!(timeout.deadline, Duration::from_millis(10));
    assert!(timeout.remaining() <= Duration::from_millis(10));
}

#[test]
fn extend_doubles_wait_until_max_timeout() {
    let mut timeout: BackoffTimeout =
        BackoffTimeout::start(Duration::from_millis(10), Duration::from_secs(1));

    // Each extension waits twice as long as the previous wait, and the last one is capped.
    let expected_deadlines_ms: [u64; 6] = [30, 70, 150, 310, 630, 1000];
    for expected_deadline_ms in expected_deadlines_ms {
        assert!(timeout.extend());
        assert_eq!(timeout.deadline, Duration::from_millis(expected_deadline_ms));
    }

    // Once the maximum timeout is reached, the timeout is no longer extended.
    assert!(!timeout.extend());
    assert_eq!(timeout.deadline, Duration::from_secs(1));
}

#[test]
fn start_caps_initial_wait_at_max_timeout() {
    let mut timeout: BackoffTimeout =
        BackoffTimeout::start(Duration::from_secs(1), Duration::from_millis(100));
    assert_eq!(timeout.deadline, Duration::from_millis(100));
    assert!(!timeout.extend());
}

#[test]
fn start_with_zero_initial_wait_waits_for_max_timeout() {
    let mut timeout: BackoffTimeout =
        BackoffTimeout::start(Duration::ZERO, Duration::from_millis(100));
    assert_eq!(timeout.deadline, Duration::from_millis(100));
    assert!(!timeout.extend());
}

#[test]
fn start_with_zero_max_timeout_expires_immediately() {
    let mut timeout: BackoffTimeout =
        BackoffTimeout::start(Duration::from_millis(10), Duration::ZERO);
    assert_eq!(timeout.remaining(), Duration::ZERO);
    assert!(!timeout.extend());
}

#[test]
fn extend_saturates_instead_of_overflowing() {
    // Doubling this wait overflows, and so does adding it to the deadline.
    let initial_wait: Duration = Duration::from_secs(u64::MAX / 2 + 1);
    let mut timeout: BackoffTimeout = BackoffTimeout::start(initial_wait, Duration::MAX);
    assert!(timeout.extend());
    assert_eq!(timeout.deadline, Duration::MAX);
    assert!(!timeout.extend());
}

#[test]
fn remaining_accounts_for_elapsed_time() {
    let wait: Duration = Duration::from_secs(1);
    let elapsed: Duration = Duration::from_millis(10);
    let timeout: BackoffTimeout = BackoffTimeout::start(wait, wait);
    thread::sleep(elapsed);
    assert!(timeout.elapsed() >= elapsed);
    assert!(timeout.remaining() <= wait - elapsed);
}
