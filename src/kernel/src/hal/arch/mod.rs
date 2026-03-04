// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(not(feature = "x86_64"))]
pub mod x86;
#[cfg(feature = "x86_64")]
pub mod x86_64;

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(all(feature = "smp", not(feature = "x86_64")))]
#[path = ""]
mod smp_feature_imports {
    pub use super::x86::Arch;
    pub use ::sys::error::Error;
}
#[cfg(all(feature = "smp", not(feature = "x86_64")))]
use smp_feature_imports::*;

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(not(feature = "x86_64"))]
pub use x86::{
    clear_task_switched,
    forge_user_stack,
    set_task_switched,
    ContextInformation,
    ExceptionInformation,
    InterruptController,
    InterruptHandler,
    InterruptNumber,
};

#[cfg(feature = "x86_64")]
pub use x86_64::{
    clear_task_switched,
    forge_user_stack,
    set_task_switched,
    ContextInformation,
    ExceptionInformation,
    InterruptController,
    InterruptHandler,
    InterruptNumber,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[cfg(all(feature = "smp", not(feature = "x86_64")))]
pub fn initialize_application_core(kstack: *const u8) -> Result<Arch, Error> {
    x86::initialize_application_core(kstack)
}
