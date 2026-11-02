// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn sigpending_reports_empty_set() {
    let mut set: sigset_t = 0xffff_ffff_ffff_ffff;
    // SAFETY: `set` points to a valid `sigset_t` for the duration of the call.
    assert_eq!(unsafe { sigpending(&mut set) }, 0);
    assert_eq!(set, 0);
}
