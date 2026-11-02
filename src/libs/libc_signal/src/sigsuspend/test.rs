// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn sigsuspend_always_reports_interruption() {
    let mask: sigset_t = 0;
    // SAFETY: `mask` points to a valid `sigset_t` for the duration of the call.
    assert_eq!(unsafe { sigsuspend(&mask) }, -1);
}
