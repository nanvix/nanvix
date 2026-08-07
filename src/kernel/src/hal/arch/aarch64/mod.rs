// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod asm;
mod cpu;
pub mod mem;

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::{
    io::{
        IoMemoryAllocator,
        IoPortAllocator,
    },
    platform::madt::MadtInfo,
};
use ::sys::error::Error;

//==================================================================================================
// Exports
//==================================================================================================

pub(crate) use asm::{
    fast_memcpy,
    fast_memset,
};
pub use cpu::{
    capture_fpu,
    forge_user_stack,
    install_fpu,
    join_kcall_result,
    prepare_kcall_restart,
    read_trap_context,
    read_user_sp,
    redirect_to_handler,
    restore_trap_context,
    returning_to_user,
    ContextInformation,
    ExceptionController,
    ExceptionInformation,
    FpuState,
    InterruptController,
    InterruptHandler,
    InterruptNumber,
    SignalCpuContext,
};

//==================================================================================================
// Structures
//==================================================================================================

pub struct Arch {
    /// GICv3 interrupt controller.
    pub controller: Option<InterruptController>,
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub unsafe fn clear_task_switched() {
    unsafe {
        cpu::enable_user_access();
    }
}

pub unsafe fn set_task_switched() {
    unsafe {
        cpu::disable_user_access();
    }
}

pub fn init(
    _ioports: &mut IoPortAllocator,
    _ioaddresses: &mut IoMemoryAllocator,
    _madt: &Option<MadtInfo>,
) -> Result<Arch, Error> {
    cpu::init()?;
    Ok(Arch {
        controller: Some(InterruptController::new()),
    })
}

#[cfg(feature = "smp")]
pub fn initialize_application_core(_kstack: *const u8) -> Result<Arch, Error> {
    cpu::initialize_application_core();
    Ok(Arch { controller: None })
}
