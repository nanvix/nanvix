// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod clock;
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

pub use clock::ticks;
pub use kcall::*;
pub use process::{
    ProcessManager,
    SleepError,
};
pub use thread::InterruptReason;

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
pub fn init(hal: &mut Hal, root: Vmem) -> Result<ProcessManager, Error> {
    info!("initializing the processor manager...");

    let interrupt_capable: bool = hal.intman.is_some();

    // Register timer handler, if interrupts are supported.
    if let Some(intman) = &mut hal.intman {
        info!("registering timer interrupt handler...");
        intman.register_handler(InterruptNumber::Timer, timer_handler)?;
    }

    // Initialize the thread manager.
    info!("initializing the thread manager...");
    let (kernel, tm): (ReadyThread, ThreadManager) = thread::init();
    let pm: ProcessManager = ProcessManager::init(interrupt_capable, kernel, root, tm);

    Ok(pm)
}
