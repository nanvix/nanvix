// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_basic() {
    assert!((scalbn(1.0, 4) - 16.0).abs() < 1e-10);
}

#[test]
fn test_negative() {
    assert!((scalbn(16.0, -4) - 1.0).abs() < 1e-10);
}
