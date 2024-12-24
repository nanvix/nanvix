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

use crate::hal::{
    arch::x86::{
        cpu::tss::TssRef,
        mem::gdt::{
            Gdt,
            GdtPtr,
        },
    },
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
    /// Global Descriptor Table (GDT).
    _gdt: Option<Gdt>,
    /// Global Descriptor Table Register (GDTR).
    pub _gdtr: Option<GdtPtr>,
    /// Task State Segment (TSS).
    pub _tss: Option<TssRef>,
    /// Interrupt controller.
    pub controller: Option<InterruptController>,
}
//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn init(
    ioports: &mut IoPortAllocator,
    ioaddresses: &mut IoMemoryAllocator,
    madt: &Option<MadtInfo>,
) -> Result<Arch, Error> {
    info!("initializing architecture-specific components...");

    // Initialize interrupt controller.
    let (gdt, gdtr, tss, controller) = cpu::init(ioports, ioaddresses, madt)?;

    Ok(Arch {
        _gdt: Some(gdt),
        _gdtr: Some(gdtr),
        _tss: Some(tss),
        controller,
    })
}

#[cfg(feature = "smp")]
pub fn initialize_application_core(kstack: *const u8) -> Result<Arch, Error> {
    let (gdt, gdtr, tss): (Gdt, GdtPtr, TssRef) = cpu::initialize_application_core(kstack)?;

    Ok(Arch {
        _gdt: Some(gdt),
        _gdtr: Some(gdtr),
        _tss: Some(tss),
        controller: None,
    })
}
