// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::kcall::KcallResult;
use ::sys::error::ErrorCode;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Stub handler for the `sigreturn()` kernel call, which restores the calling thread's context
/// after a signal handler returns.
///
/// This is scaffolding for the signal subsystem and is not yet implemented: it always fails with
/// [`ErrorCode::InvalidSysCall`]. A later phase of the signals effort replaces this stub with the
/// real handler.
///
/// # Returns
///
/// Always returns a [`KcallResult`] carrying [`ErrorCode::InvalidSysCall`].
///
pub fn sigreturn() -> KcallResult {
    KcallResult::Error(ErrorCode::InvalidSysCall.into())
}
