// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::difftime;
use ::sysapi::sys_types::time_t;

#[test]
fn test_difftime_positive() {
    let result: f64 = difftime(100 as time_t, 50 as time_t);
    assert!((result - 50.0).abs() < f64::EPSILON, "Expected 50.0, got {result}");
}

#[test]
fn test_difftime_negative() {
    let result: f64 = difftime(50 as time_t, 100 as time_t);
    assert!((result - (-50.0)).abs() < f64::EPSILON, "Expected -50.0, got {result}");
}

#[test]
fn test_difftime_zero() {
    let result: f64 = difftime(100 as time_t, 100 as time_t);
    assert!((result - 0.0).abs() < f64::EPSILON, "Expected 0.0, got {result}");
}

#[test]
fn test_difftime_large_values() {
    let t1: time_t = 1_000_000_000;
    let t0: time_t = 0;
    let result: f64 = difftime(t1, t0);
    assert!((result - 1_000_000_000.0).abs() < f64::EPSILON, "Expected 1e9, got {result}");
}
