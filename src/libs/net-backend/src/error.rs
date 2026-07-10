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
    pub fn is_would_block(&self) -> bool {
        matches!(self, NetError::Errno(ErrorCode::TryAgain))
    }

    /// Returns `true` if this error indicates that a non-blocking `connect()` has been initiated
    /// or is still in progress (`EINPROGRESS` / `EALREADY`).
    ///
    /// Completion is signalled later by the socket becoming writable.
    pub fn is_in_progress(&self) -> bool {
        matches!(
            self,
            NetError::Errno(ErrorCode::OperationInProgress | ErrorCode::OperationAlreadyInProgress)
        )
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

    /// Tests that `is_in_progress` is true for pending non-blocking connect errnos.
    #[test]
    fn is_in_progress_matches_pending_connect_errors() {
        assert!(NetError::Errno(ErrorCode::OperationInProgress).is_in_progress());
        assert!(NetError::Errno(ErrorCode::OperationAlreadyInProgress).is_in_progress());
        assert!(!NetError::Errno(ErrorCode::TryAgain).is_in_progress());
        assert!(!NetError::Interrupted.is_in_progress());
    }
}
