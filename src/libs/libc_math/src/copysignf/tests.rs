// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_positive_to_negative() {
    assert!((copysignf(3.0, -1.0) - (-3.0)).abs() < 1e-5);
}

#[test]
fn test_negative_to_positive() {
    assert!((copysignf(-3.0, 1.0) - 3.0).abs() < 1e-5);
}
