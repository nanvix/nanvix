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

use crate::{
    error::Error,
    hal::{
        arch::x86::{
            cpu::{
                pit,
                tss::TssRef,
            },
            mem::gdt::{
                Gdt,
                GdtPtr,
            },
            pit::Pit,
        },
        io::{
            IoMemoryAllocator,
            IoPortAllocator,
        },
    },
};
use cpu::madt::madt::MadtInfo;

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
    pub gdtr: Option<GdtPtr>,
    /// Task State Segment (TSS).
    pub tss: Option<TssRef>,
    /// Interrupt controller.
    pub controller: Option<InterruptController>,
    /// Programmable Interval Timer (PIT).
    pub pit: Option<Pit>,
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn init(
    ioports: &mut IoPortAllocator,
    ioaddresses: &mut IoMemoryAllocator,
    madt: Option<MadtInfo>,
) -> Result<Arch, Error> {
    info!("initializing architecture-specific components...");

    // Initialize interrupt controller.
    let (gdt, gdtr, tss, controller, pit) = cpu::init(ioports, ioaddresses, madt)?;

    Ok(Arch {
        _gdt: Some(gdt),
        gdtr: Some(gdtr),
        tss: Some(tss),
        controller: Some(controller),
        pit: Some(pit),
    })
}
