// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod capability;
mod manager;
pub(crate) mod state;

//==================================================================================================
// Exports
//==================================================================================================

pub use manager::{
    ExceptionGuard,
    ProcessManager,
    SigReturnFailure,
    SignalDeliveryOutcome,
    SleepError,
    SyncSignalOutcome,
};
pub use state::exception_to_signal;

//==================================================================================================
// Tests
//==================================================================================================

/// Runs all in-kernel unit tests for the process module.
#[cfg(feature = "test")]
pub(super) fn test() -> bool {
    let mut passed: bool = true;
    passed &= manager::test();
    passed &= state::test();
    passed
}
