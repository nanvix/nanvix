// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod context;
mod exception;
mod idt;
mod interrupt;

#[cfg(feature = "smp")]
mod clock;

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
        MadtInfo,
    },
    io::{
        IoMemoryAllocator,
        IoPortAllocator,
    },
};
use ::sys::{
    arch::cpu::cpuid,
    error::Error,
};

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

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn init(
    ioports: &mut IoPortAllocator,
    ioaddresses: &mut IoMemoryAllocator,
    madt: &Option<MadtInfo>,
) -> Result<(Gdt, GdtPtr, TssRef, Option<InterruptController>), Error> {
    extern "C" {
        static kstack: u8;
    }

    // Check if the CPU has the cpuid instruction.
    if ::sys::arch::cpu::cpuid::has_cpuid() {
        info!("CPU features:");
        info!("- has fpu:   {}", cpuid::has_fpu());
        info!("- has vme:   {}", cpuid::has_vme());
        info!("- has de:    {}", cpuid::has_de());
        info!("- has pse:   {}", cpuid::has_pse());
        info!("- has tsc:   {}", cpuid::has_tsc());
        info!("- has msr:   {}", cpuid::has_msr());
        info!("- has pae:   {}", cpuid::has_pae());
        info!("- has mce:   {}", cpuid::has_mce());
        info!("- has cx8:   {}", cpuid::has_cx8());
        info!("- has apic:  {}", cpuid::has_apic());
        info!("- has sep:   {}", cpuid::has_sep());
        info!("- has mtrr:  {}", cpuid::has_mtrr());
        info!("- has pge:   {}", cpuid::has_pge());
        info!("- has mca:   {}", cpuid::has_mca());
        info!("- has cmov:  {}", cpuid::has_cmov());
        info!("- has pat:   {}", cpuid::has_pat());
        info!("- has pse36: {}", cpuid::has_pse36());
        info!("- has psn:   {}", cpuid::has_psn());
        info!("- has clfsh: {}", cpuid::has_clflush());
        info!("- has ds:    {}", cpuid::has_ds());
        info!("- has acpi:  {}", cpuid::has_acpi());
        info!("- has mmx:   {}", cpuid::has_mmx());
        info!("- has fxsr:  {}", cpuid::has_fxsr());
        info!("- has sse:   {}", cpuid::has_sse());
        info!("- has sse2:  {}", cpuid::has_sse2());
        info!("- has ss:    {}", cpuid::has_ss());
        info!("- has htt:   {}", cpuid::has_htt());
        info!("- has tm:    {}", cpuid::has_tm());
        info!("- has ia64:  {}", cpuid::has_ia64());
        info!("- has pbe:   {}", cpuid::has_pbe());
    }

    let (gdt, gdtr, tss): (Gdt, GdtPtr, TssRef) = unsafe { Gdt::init(&kstack)? };
    unsafe { idt::init() };

    let controller: Option<InterruptController> = match interrupt::init(ioports, ioaddresses, madt)
    {
        Ok(controller) => Some(controller),
        Err(e) => {
            warn!("failed to initialize interrupt controller (error={:?})", e);
            None
        },
    };

    Ok((gdt, gdtr, tss, controller))
}

#[cfg(feature = "smp")]
pub fn initialize_application_core(kstack: *const u8) -> Result<(Gdt, GdtPtr, TssRef), Error> {
    let (gdt, gdtr, tss): (Gdt, GdtPtr, TssRef) = unsafe { Gdt::init(kstack)? };
    unsafe { idt::load() };

    Ok((gdt, gdtr, tss))
}
