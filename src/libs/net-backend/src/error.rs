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
