// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_alias_of_lgamma() {
    // gamma() is the legacy alias for lgamma().
    assert_eq!(gamma(3.0), crate::lgamma::lgamma(3.0));
    assert_eq!(gamma(5.0), crate::lgamma::lgamma(5.0));
}

#[test]
fn test_nan() {
    assert!(gamma(f64::NAN).is_nan());
}
