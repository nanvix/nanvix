// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

pub mod x86;

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(feature = "smp")]
#[path = ""]
mod smp_feature_imports {
    pub use super::x86::Arch;
    pub use ::sys::error::Error;
}
#[cfg(feature = "smp")]
use smp_feature_imports::*;

//==================================================================================================
// Exports
//==================================================================================================

pub use x86::{
    forge_user_stack,
    ContextInformation,
    ExceptionInformation,
    InterruptController,
    InterruptHandler,
    InterruptNumber,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[cfg(feature = "smp")]
pub fn initialize_application_core(kstack: *const u8) -> Result<Arch, Error> {
    x86::initialize_application_core(kstack)
}
