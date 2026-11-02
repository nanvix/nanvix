// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn raise_reports_failure_on_host() {
    // The host build cannot deliver signals, so raise() must fail rather than reference any
    // guest-only symbol. This guards against reintroducing an unconditional FFI call that
    // would break the host build.
    assert_eq!(unsafe { raise(2) }, -1);
}
