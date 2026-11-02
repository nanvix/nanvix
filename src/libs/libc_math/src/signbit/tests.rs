// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_signbitf() {
    assert_eq!(__signbitf(1.0), 0);
    assert_eq!(__signbitf(-1.0), 1);
    assert_eq!(__signbitf(-0.0), 1);
    assert_eq!(__signbitf(0.0), 0);
}

#[test]
fn test_signbitd() {
    assert_eq!(__signbitd(1.0), 0);
    assert_eq!(__signbitd(-1.0), 1);
    assert_eq!(__signbitd(-0.0), 1);
    assert_eq!(__signbitd(0.0), 0);
}
