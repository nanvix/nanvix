// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

// ---- sa_data roundtrip ----------------------------------------------------------------------

/// Tests that `sa_data_from_u8` / `sa_data_to_u8` roundtrips zeros.
#[test]
fn sa_data_roundtrip_zeros() {
    let input: [u8; 14] = [0u8; 14];
    let result: [u8; 14] = sa_data_to_u8(sa_data_from_u8(input));
    assert_eq!(result, input, "zero roundtrip should be identity");
}

/// Tests that `sa_data_from_u8` / `sa_data_to_u8` roundtrips non-zero data.
#[test]
fn sa_data_roundtrip_nonzero() {
    let input: [u8; 14] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];
    let result: [u8; 14] = sa_data_to_u8(sa_data_from_u8(input));
    assert_eq!(result, input, "non-zero roundtrip should be identity");
}

// ---- Boolean helpers ------------------------------------------------------------------------

/// Tests that `is_interrupted` returns true for `EINTR`.
#[test]
fn is_interrupted_true_for_eintr() {
    assert!(is_interrupted(libc::EINTR), "EINTR should be detected as interrupted");
}

/// Tests that `is_interrupted` returns false for zero.
#[test]
fn is_interrupted_false_for_zero() {
    assert!(!is_interrupted(0), "zero should not be interrupted");
}

/// Tests that `socket_failed` returns true for `INVALID_SOCKET` (-1).
#[test]
fn socket_failed_true_for_invalid() {
    assert!(socket_failed(INVALID_SOCKET), "INVALID_SOCKET should indicate failure");
}

/// Tests that `socket_failed` returns false for a valid fd.
#[test]
fn socket_failed_false_for_valid() {
    assert!(!socket_failed(3), "valid fd should not indicate failure");
}

// ---- Handle conversion ----------------------------------------------------------------------

/// Tests that `raw_to_i32` / `i32_to_raw` roundtrips on Unix.
#[test]
fn handle_conversion_roundtrip() {
    let original: i32 = 42;
    let raw: RawSocket = i32_to_raw(original);
    let back: i32 = raw_to_i32(raw);
    assert_eq!(back, original, "handle roundtrip should be identity");
}
