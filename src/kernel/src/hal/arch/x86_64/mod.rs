// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

pub mod cpu;
pub mod mem;

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::io::{
    IoMemoryAllocator,
    IoPortAllocator,
};
use crate::hal::platform::madt::MadtInfo;
use ::sys::error::Error;

//==================================================================================================
// Exports
//==================================================================================================

pub use cpu::{
    forge_user_stack,
    ContextInformation,
    ExceptionInformation,
    InterruptController,
    InterruptHandler,
    InterruptNumber,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that describes the architecture-specific components.
///
pub struct Arch {
    /// Global Descriptor Table Register (GDTR).
    pub _gdtr: Option<mem::gdt::GdtPtr>,
    /// Task State Segment (TSS).
    pub _tss: Option<cpu::tss::TssRef>,
    /// Interrupt controller.
    pub controller: Option<InterruptController>,
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Clears the CR0.TS flag to enable FPU/SSE instructions.
///
pub unsafe fn clear_task_switched() {
    // TODO: Implement for x86_64.
    // In long mode, CR0.TS works the same way.
}

///
/// # Description
///
/// Sets the CR0.TS flag to disable FPU/SSE instructions.
///
pub unsafe fn set_task_switched() {
    // TODO: Implement for x86_64.
}

pub fn init(
    ioports: &mut IoPortAllocator,
    ioaddresses: &mut IoMemoryAllocator,
    madt: &Option<MadtInfo>,
) -> Result<Arch, Error> {
    info!("initializing x86_64 architecture-specific components...");

    let (gdtr, tss, controller) = cpu::init(ioports, ioaddresses, madt)?;

    Ok(Arch {
        _gdtr: Some(gdtr),
        _tss: Some(tss),
        controller,
    })
}
