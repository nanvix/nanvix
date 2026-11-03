// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_basic() {
    assert!((scalbnf(1.0, 4) - 16.0).abs() < 1e-5);
}
