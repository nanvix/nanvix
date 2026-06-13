// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod capability;
mod manager;
mod state;

//==================================================================================================
// Exports
//==================================================================================================

pub use manager::{
    ExceptionGuard,
    ProcessManager,
    SleepError,
};

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
