// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_ten() {
    assert!((log10f(10.0) - 1.0).abs() < 1e-3);
}

#[test]
fn test_one() {
    assert!((log10f(1.0)).abs() < 1e-4);
}
