// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_two() {
    assert!((log2f(2.0) - 1.0).abs() < 1e-3);
}

#[test]
fn test_one() {
    assert!((log2f(1.0)).abs() < 1e-4);
}
