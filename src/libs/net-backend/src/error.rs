// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::ErrorCode;

//==================================================================================================
// NetError
//==================================================================================================

/// Errors returned by `NetBackend` operations.
#[derive(Debug)]
pub enum NetError {
    /// The operation was interrupted by a signal (EINTR).
    Interrupted,
    /// The operation failed with a specific error code.
    Errno(ErrorCode),
}

impl NetError {
    /// Returns `true` if this error indicates that a non-blocking operation could not be completed
    /// immediately and would block (`EAGAIN` / `EWOULDBLOCK`).
    ///
    /// The epoll reactor uses this to park the operation on socket readiness instead of surfacing
    /// it as a fatal error. On blocking sockets this condition never arises, so existing callers
    /// are unaffected.
    pub fn is_would_block(&self) -> bool {
        matches!(self, NetError::Errno(ErrorCode::TryAgain))
    }

    /// Returns `true` if this error indicates that a non-blocking `connect()` has been initiated
    /// and is still in progress (`EINPROGRESS`).
    ///
    /// Completion is signalled later by the socket becoming writable.
    pub fn is_in_progress(&self) -> bool {
        matches!(self, NetError::Errno(ErrorCode::OperationInProgress))
    }
}

impl core::fmt::Display for NetError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NetError::Interrupted => write!(f, "operation interrupted (EINTR)"),
            NetError::Errno(code) => write!(f, "network error: {code}"),
        }
    }
}

impl std::error::Error for NetError {}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod test {
    use super::*;

    /// Tests that `is_would_block` is true only for the `TryAgain` errno.
    #[test]
    fn is_would_block_matches_tryagain() {
        assert!(NetError::Errno(ErrorCode::TryAgain).is_would_block());
        assert!(!NetError::Errno(ErrorCode::OperationInProgress).is_would_block());
        assert!(!NetError::Errno(ErrorCode::ConnectionRefused).is_would_block());
        assert!(!NetError::Interrupted.is_would_block());
    }

    /// Tests that `is_in_progress` is true only for the `OperationInProgress` errno.
    #[test]
    fn is_in_progress_matches_operation_in_progress() {
        assert!(NetError::Errno(ErrorCode::OperationInProgress).is_in_progress());
        assert!(!NetError::Errno(ErrorCode::TryAgain).is_in_progress());
        assert!(!NetError::Interrupted.is_in_progress());
    }
}
