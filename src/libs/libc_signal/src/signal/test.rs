// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn sig_err_is_non_null_sentinel() {
    // SIG_ERR must be a non-null value so that C callers can distinguish it from a real
    // handler or from SIG_DFL (null).
    assert_ne!(SIG_ERR, SIG_DFL, "SIG_ERR must be a non-null sentinel");
}

#[test]
fn is_valid_signum_accepts_in_range_signals() {
    assert!(is_valid_signum(1));
    assert!(is_valid_signum(SIG_MAX));
}

#[test]
fn is_valid_signum_rejects_out_of_range_signals() {
    assert!(!is_valid_signum(0));
    assert!(!is_valid_signum(-1));
    assert!(!is_valid_signum(SIG_MAX + 1));
}

#[test]
fn signal_rejects_invalid_signum() {
    // Invalid signal numbers must fail with SIG_ERR (a non-null sentinel) rather than touch
    // kernel state.
    assert_eq!(unsafe { signal(0, SIG_DFL) }, SIG_ERR);
    assert_eq!(unsafe { signal(-1, SIG_DFL) }, SIG_ERR);
    assert_eq!(unsafe { signal(SIG_MAX + 1, SIG_DFL) }, SIG_ERR);
}
