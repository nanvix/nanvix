// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::time::SystemTime;
use ::core::time::Duration;

//==================================================================================================
// Unit Tests
//==================================================================================================

#[test]
fn test_system_time_new_valid() {
    let time = SystemTime::new(10, 500_000_000);
    assert!(time.is_some());
    let time = time.unwrap();
    assert_eq!(time.seconds(), 10);
    assert_eq!(time.nanoseconds(), 500_000_000);
}

#[test]
fn test_system_time_new_invalid() {
    let time = SystemTime::new(10, 1_000_000_000);
    assert!(time.is_none());
}

#[test]
fn test_checked_add_duration_no_overflow() {
    let time = SystemTime::new(10, 500_000_000).unwrap();
    let duration = Duration::new(5, 200_000_000);
    let result = time.checked_add_duration(&duration);
    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.seconds(), 15);
    assert_eq!(result.nanoseconds(), 700_000_000);
}

#[test]
fn test_checked_add_duration_with_overflow() {
    let time = SystemTime::new(10, 900_000_000).unwrap();
    let duration = Duration::new(5, 200_000_000);
    let result = time.checked_add_duration(&duration);
    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.seconds(), 16);
    assert_eq!(result.nanoseconds(), 100_000_000);
}

#[test]
fn test_checked_sub_duration_no_underflow() {
    let time = SystemTime::new(10, 500_000_000).unwrap();
    let duration = Duration::new(5, 200_000_000);
    let result = time.checked_sub_duration(&duration);
    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.seconds(), 5);
    assert_eq!(result.nanoseconds(), 300_000_000);
}

#[test]
fn test_checked_sub_duration_with_underflow() {
    let time = SystemTime::new(10, 100_000_000).unwrap();
    let duration = Duration::new(5, 200_000_000);
    let result = time.checked_sub_duration(&duration);
    assert!(result.is_none());
}

#[test]
fn test_checked_sub_time_no_underflow() {
    let time1 = SystemTime::new(10, 500_000_000).unwrap();
    let time2 = SystemTime::new(5, 200_000_000).unwrap();
    let result = time1.checked_sub(&time2);
    assert!(result.is_ok());
    let result = result.unwrap();
    assert_eq!(result.as_secs(), 5);
    assert_eq!(result.subsec_nanos(), 300_000_000);
}
