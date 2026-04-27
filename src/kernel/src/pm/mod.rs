// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

pub(crate) mod clock;
mod kcall;
mod process;
pub mod sync;
pub mod thread;

//==================================================================================================
// Imports
//==================================================================================================

use self::clock::timer_handler;
use crate::{
    hal::{
        arch::InterruptNumber,
        mem::VirtualAddress,
        Hal,
    },
    mm::Vmem,
    pm::thread::{
        ReadyThread,
        ThreadManager,
    },
};
use ::core::sync::atomic::Ordering;
use ::sys::{
    error::Error,
    pm::ProcessIdentifier,
};

//==================================================================================================
// Constants
//==================================================================================================

// Use relaxed ordering for all atomic operations to mitigate synchronization overhead. It is safe
// to use this ordering semantics because Nanvix is a single-core system, and the kernel runs with
// interrupts disabled.
const ORDER: Ordering = Ordering::Relaxed;

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(not(feature = "hyperlight"))]
pub use clock::ticks;
pub use kcall::*;
pub use process::{
    ExceptionGuard,
    ProcessManager,
    SleepError,
};
pub use thread::InterruptReason;

///
/// # Description
///
/// Interrupt handler for IKC (inter-kernel communication) notifications.
///
/// The VMM injects this interrupt (IRQ 9) whenever a new message is enqueued for the guest.
/// The handler polls IKC messages immediately so the kernel does not have to wait for the
/// next timer tick to process newly arrived messages.
///
/// # Safety
///
/// Called from interrupt context with interrupts disabled on a single-core system.
/// At that point the CPU was halted (HLT), so no mutable references to kernel state
/// are alive and it is safe to re-enter the process manager.
///
#[cfg(feature = "microvm")]
unsafe fn ikc_interrupt_handler(_intnum: InterruptNumber) {
    crate::kcall::poll_ikc_messages();
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn copy_from_user<T>(
    pm: &mut ProcessManager,
    pid: ProcessIdentifier,
    dst: &mut T,
    src: *const T,
) -> Result<(), Error> {
    let dst: VirtualAddress = VirtualAddress::from_raw_value(dst as *mut T as usize);
    let src: VirtualAddress = VirtualAddress::from_raw_value(src as usize);
    let size: usize = core::mem::size_of::<T>();

    pm.vmcopy_from_user(pid, dst, src, size)
}

pub fn copy_to_user<T>(
    pm: &mut ProcessManager,
    pid: ProcessIdentifier,
    dst: *mut T,
    src: &T,
) -> Result<(), Error> {
    let dst: VirtualAddress = VirtualAddress::from_raw_value(dst as usize);
    let src: VirtualAddress = VirtualAddress::from_raw_value(src as *const T as usize);
    let size: usize = core::mem::size_of::<T>();

    pm.vmcopy_to_user(pid, dst, src, size)
}

/// Initializes the processor manager.
pub fn init(root: Vmem) -> Result<(), Error> {
    info!("initializing the processor manager...");

    // SAFETY: the hardware abstraction layer is initialized and access is synchronized.
    let hal: &mut Hal = unsafe { Hal::get_mut() };

    let interrupt_capable: bool = hal.is_interrupt_capable();

    // Register timer handler, if interrupts are supported.
    if let Some(intman) = hal.intman() {
        info!("registering timer interrupt handler...");
        intman.register_handler(InterruptNumber::Timer, timer_handler)?;

        // Register a dedicated IKC notification handler (microvm only). The VMM injects
        // IRQ 9 whenever a new IKC message is enqueued, waking the kernel from HLT.
        #[cfg(feature = "microvm")]
        {
            info!("registering IKC interrupt handler...");
            intman.register_handler(InterruptNumber::Ikc, ikc_interrupt_handler)?;
        }
    }

    // Initialize the thread manager.
    info!("initializing the thread manager...");
    let (kernel, tm): (ReadyThread, ThreadManager) = thread::init();
    ProcessManager::init(interrupt_capable, kernel, root, tm);

    Ok(())
}
