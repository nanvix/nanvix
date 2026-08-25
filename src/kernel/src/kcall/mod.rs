// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod dispatcher;
mod handler;
mod kcall_error;
mod kcall_result;
mod kcall_success;

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(feature = "test")]
pub(crate) use handler::drain_lifecycle_wakeup;
pub use handler::kcall_handler as handler;
pub use kcall_error::KcallError;
pub use kcall_result::KcallResult;
pub use kcall_success::KcallSuccess;
