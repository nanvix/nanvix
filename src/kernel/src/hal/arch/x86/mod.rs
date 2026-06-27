// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

pub mod asm;
pub mod cpu;
pub mod mem;

pub(crate) use asm::{
    fast_memcpy,
    fast_memset,
};

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::{
    arch::x86::{
        cpu::{
            tss::TssRef,
            XapicTimer,
        },
        mem::gdt::GdtPtr,
    },
    io::{
        IoMemoryAllocator,
        IoPortAllocator,
    },
    platform::madt::MadtInfo,
};
use ::arch::cpu::cr0::{
    Cr0Register,
    TaskSwitchedFlag,
};
use ::sys::error::Error;

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(feature = "test")]
pub use cpu::split_kcall_result;
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
    ExceptionInformation,
    InterruptController,
    InterruptHandler,
    InterruptNumber,
    SignalCpuContext,
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
    pub _gdtr: Option<GdtPtr>,
    /// Task State Segment (TSS).
    pub _tss: Option<TssRef>,
    /// Interrupt controller.
    pub controller: Option<InterruptController>,
    /// xAPIC timer (platform-owned, used only in PIC + xAPIC timer mode).
    pub _xapic_timer: Option<XapicTimer>,
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Clears the CR0.TS flag to enable FPU/SSE instructions.
///
/// # Safety
///
/// It is unsafe to call this function because it executes privileged instructions.
///
/// It is safe to call this function if the following conditions are met:
/// - Calls to this function are synchronized.
/// - The caller runs at processor privilege level 0.
///
pub unsafe fn clear_task_switched() {
    let mut cr0: Cr0Register = Cr0Register::read();
    cr0.task_switched = TaskSwitchedFlag::Clear;
    cr0.write();
}

///
/// # Description
///
/// Sets the CR0.TS flag to disable FPU/SSE instructions.
///
/// # Safety
///
/// It is unsafe to call this function because it executes privileged instructions.
///
/// It is safe to call this function if the following conditions are met:
/// - Calls to this function are synchronized.
/// - The caller runs at processor privilege level 0.
///
pub unsafe fn set_task_switched() {
    let mut cr0: Cr0Register = Cr0Register::read();
    cr0.task_switched = TaskSwitchedFlag::Set;
    cr0.write();
}

pub fn init(
    ioports: &mut IoPortAllocator,
    ioaddresses: &mut IoMemoryAllocator,
    madt: &Option<MadtInfo>,
) -> Result<Arch, Error> {
    info!("initializing architecture-specific components...");

    // Initialize CPU subsystem (GDT, IDT, interrupt controller, xAPIC timer).
    let (gdtr, tss, controller, xapic_timer) = cpu::init(ioports, ioaddresses, madt)?;

    Ok(Arch {
        _gdtr: Some(gdtr),
        _tss: Some(tss),
        controller,
        _xapic_timer: xapic_timer,
    })
}

#[cfg(feature = "smp")]
pub fn initialize_application_core(kstack: *const u8) -> Result<Arch, Error> {
    let (gdtr, tss): (GdtPtr, TssRef) = cpu::initialize_application_core(kstack)?;

    Ok(Arch {
        _gdtr: Some(gdtr),
        _tss: Some(tss),
        controller: None,
        _xapic_timer: None,
    })
}
