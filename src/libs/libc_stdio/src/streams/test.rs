// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::{
    stderr,
    stdin,
    stdout,
};

#[test]
fn standard_streams_have_expected_descriptors() {
    // SAFETY: the accessors return valid pointers to the static standard streams.
    unsafe {
        assert_eq!((*stdin()).fd, 0);
        assert_eq!((*stdout()).fd, 1);
        assert_eq!((*stderr()).fd, 2);
    }
}

#[test]
fn standard_streams_start_without_errors_pushback_or_orientation() {
    // SAFETY: the accessors return valid pointers to the static standard streams.
    unsafe {
        let s: *mut super::FILE = stdin();
        assert_eq!((*s).eof, 0);
        assert_eq!((*s).error, 0);
        assert_eq!((*s).ungetc_buf, -1);
        assert_eq!((*s).orientation, 0);
    }
}

#[test]
fn standard_streams_are_distinct_and_stable() {
    // SAFETY: the accessors return stable pointers to distinct static streams.
    unsafe {
        assert!(!core::ptr::eq(stdin(), stdout()));
        assert!(!core::ptr::eq(stdout(), stderr()));
        assert!(core::ptr::eq(stdin(), stdin()));
    }
}
