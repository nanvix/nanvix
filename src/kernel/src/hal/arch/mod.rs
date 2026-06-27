// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(target_arch = "x86")]
pub mod x86;

#[cfg(target_arch = "x86_64")]
#[path = "x86_64/mod.rs"]
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
    capture_fpu,
    clear_task_switched,
    forge_user_stack,
    install_fpu,
    join_kcall_result,
    prepare_kcall_restart,
    read_trap_context,
    read_user_sp,
    redirect_to_handler,
    restore_trap_context,
    returning_to_user,
    set_task_switched,
    ContextInformation,
    ExceptionInformation,
    InterruptController,
    InterruptHandler,
    InterruptNumber,
    SignalCpuContext,
};

#[cfg(all(target_arch = "x86", feature = "test"))]
pub use x86::split_kcall_result;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[cfg(feature = "smp")]
pub fn initialize_application_core(kstack: *const u8) -> Result<Arch, Error> {
    x86::initialize_application_core(kstack)
}
