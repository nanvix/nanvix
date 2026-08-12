// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod kcall;
mod manager;

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;

//==================================================================================================
// Exports
//==================================================================================================

pub use kcall::*;
pub use manager::{
    EventManager,
    EventOwnership,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn init() -> Result<(), Error> {
    manager::init()
}

///
/// # Description
///
/// Runs the event subsystem in-kernel tests.
///
/// # Returns
///
/// `true` if every test passed, `false` otherwise.
///
#[cfg(feature = "test")]
pub fn test() -> bool {
    manager::test::test()
}

/// Runs ordered-delivery integration tests after process-manager initialization.
#[cfg(feature = "test")]
pub fn test_delivery_integration() -> bool {
    manager::test::test_delivery_integration()
}
