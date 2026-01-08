// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod dispatcher;
mod handler;
mod kcall_args;
mod kcall_error;
mod kcall_result;
mod kcall_success;
mod scoreboard;

//==================================================================================================
// Exports
//==================================================================================================

pub use handler::kcall_handler as handler;
pub use kcall_args::KcallArgs;
pub use kcall_error::KcallError;
pub use kcall_result::KcallResult;
pub use kcall_success::KcallSuccess;
pub(crate) use scoreboard::ScoreBoard;

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn init() {
    info!("initializing kernel call handler...");
    ScoreBoard::init();
}
