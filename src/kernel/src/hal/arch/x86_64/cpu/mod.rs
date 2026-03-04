// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod context;
mod exception;
mod idt;
mod interrupt;
mod fpu;

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::{
    arch::x86_64::{
        cpu::tss::TssRef,
        mem::gdt::{
            Gdt,
            GdtPtr,
        },
        MadtInfo,
    },
    io::{
        IoMemoryAllocator,
        IoPortAllocator,
    },
};
use ::sys::error::Error;

//==================================================================================================
// Exports
//==================================================================================================

pub use context::ContextInformation;
pub use exception::{
    ExceptionController,
    ExceptionInformation,
};
pub use interrupt::{
    forge_user_stack,
    InterruptController,
    InterruptHandler,
    InterruptNumber,
};
pub mod tss;
pub use fpu::FpuState;

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn init(
    ioports: &mut IoPortAllocator,
    ioaddresses: &mut IoMemoryAllocator,
    madt: &Option<MadtInfo>,
) -> Result<(GdtPtr, TssRef, Option<InterruptController>), Error> {
    unsafe extern "C" {
        static kstack: u8;
    }

    // Initialize FPU/SSE.
    unsafe { fpu::init() };

    let (gdtr, tss): (GdtPtr, TssRef) = unsafe { Gdt::init(&kstack)? };
    unsafe { idt::init() };

    let controller: Option<InterruptController> = match interrupt::init(ioports, ioaddresses, madt)
    {
        Ok(controller) => Some(controller),
        Err(e) => {
            warn!("failed to initialize interrupt controller (error={:?})", e);
            None
        },
    };

    Ok((gdtr, tss, controller))
}
