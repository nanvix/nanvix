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

#[cfg(target_arch = "aarch64")]
pub mod aarch64;

#[cfg(target_arch = "aarch64")]
pub use aarch64 as native;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub use x86 as native;

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(feature = "smp")]
#[path = ""]
mod smp_feature_imports {
    #[cfg(target_arch = "aarch64")]
    pub use super::aarch64::Arch;
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub use super::x86::Arch;
    pub use ::sys::error::Error;
}
#[cfg(feature = "smp")]
use smp_feature_imports::*;

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(target_arch = "aarch64")]
pub use aarch64::{
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
    ExceptionController,
    ExceptionInformation,
    FpuState,
    InterruptController,
    InterruptHandler,
    InterruptNumber,
    SignalCpuContext,
};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
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
    ExceptionController,
    ExceptionInformation,
    FpuState,
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
    #[cfg(target_arch = "aarch64")]
    {
        aarch64::initialize_application_core(kstack)
    }
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        x86::initialize_application_core(kstack)
    }
}
