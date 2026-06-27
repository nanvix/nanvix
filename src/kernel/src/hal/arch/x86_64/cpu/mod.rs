// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod context;
mod exception;
pub(crate) mod idt;
mod interrupt;
mod sigframe;

#[cfg(feature = "smp")]
#[path = "../../shared/cpu/clock.rs"]
mod clock;

#[path = "../../shared/cpu/fpu.rs"]
mod fpu;

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
use ::arch::cpu::cpuid;
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Exports
//==================================================================================================

pub use context::{
    ContextInformation,
    SignalCpuContext,
};
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
pub use sigframe::{
    join_kcall_result,
    read_trap_context,
    read_user_sp,
    redirect_to_handler,
    restore_trap_context,
    returning_to_user,
};
pub mod tss;
pub use fpu::{
    capture_fpu,
    install_fpu,
    FpuState,
};

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

    // Check if the CPU has the cpuid instruction.
    if ::arch::cpu::cpuid::has_cpuid() {
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

        // Check if required hardware features are supported.
        if !((cpuid::has_sse() || cpuid::has_sse2()) && cpuid::has_fxsr()) {
            let reason: &str = "cpu does not support SSE or SSE2 features with FXSR";
            error!("{reason}");
            return Err(Error::new(ErrorCode::NoSuchEntry, reason));
        }

        // SAFETY: The following conditions are met:
        // - Calls to this function are synchronized.
        // - This function runs on a processor that supports SSE or SSE2 features.
        // - This function runs on a processor that supports FXSAVE and FXRSTOR instructions.
        // - This function runs at processor privilege level 0.
        unsafe { fpu::init() };
    }

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

#[cfg(feature = "smp")]
pub fn initialize_application_core(kstack: *const u8) -> Result<(GdtPtr, TssRef), Error> {
    let (gdtr, tss): (GdtPtr, TssRef) = unsafe { Gdt::init(kstack)? };
    unsafe { idt::load() };

    Ok((gdtr, tss))
}
