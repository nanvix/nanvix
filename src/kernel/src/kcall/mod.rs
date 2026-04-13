// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod dispatcher;
pub(crate) mod handler;
mod kcall_error;
mod kcall_result;
mod kcall_success;

//==================================================================================================
// Exports
//==================================================================================================

pub use handler::kcall_handler as handler;
#[cfg(feature = "microvm")]
pub use handler::poll_ikc_messages;
pub use kcall_error::KcallError;
pub use kcall_result::KcallResult;
pub use kcall_success::KcallSuccess;
