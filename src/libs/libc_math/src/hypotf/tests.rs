// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_345() {
    assert!((hypotf(3.0, 4.0) - 5.0).abs() < 1e-4);
}

#[test]
fn test_inf() {
    assert_eq!(hypotf(f32::INFINITY, 1.0), f32::INFINITY);
}
