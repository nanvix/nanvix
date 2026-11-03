// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

/// Tests that `is_would_block` is true only for the `TryAgain` errno.
#[test]
fn is_would_block_matches_tryagain() {
    assert!(NetError::Errno(ErrorCode::TryAgain).is_would_block());
    assert!(!NetError::Errno(ErrorCode::OperationInProgress).is_would_block());
    assert!(!NetError::Errno(ErrorCode::ConnectionRefused).is_would_block());
    assert!(!NetError::Interrupted.is_would_block());
}

/// Tests that `is_in_progress` is true for pending non-blocking connect errnos.
#[test]
fn is_in_progress_matches_pending_connect_errors() {
    assert!(NetError::Errno(ErrorCode::OperationInProgress).is_in_progress());
    assert!(NetError::Errno(ErrorCode::OperationAlreadyInProgress).is_in_progress());
    assert!(!NetError::Errno(ErrorCode::TryAgain).is_in_progress());
    assert!(!NetError::Interrupted.is_in_progress());
}
