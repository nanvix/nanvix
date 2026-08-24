// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

pub(crate) mod clock;
mod kcall;
mod process;
pub mod sync;
#[cfg(feature = "test")]
mod test;
pub mod thread;
mod uncommitted_message;
mod uncommitted_message_token;

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
#[cfg(feature = "test")]
pub(crate) use process::new_test_delivery_sequence;
pub(crate) use process::DeliverySequence;
pub use process::{
    exception_to_signal,
    ExceptionGuard,
    ProcessManager,
    SigReturnFailure,
    SignalDeliveryOutcome,
    SleepError,
    SyncSignalOutcome,
};
pub use thread::{
    InterruptReason,
    KcallRestart,
};
#[cfg(feature = "test")]
pub(crate) use uncommitted_message::new_test_message;
pub(crate) use uncommitted_message::UncommittedMessage;
pub(crate) use uncommitted_message_token::UncommittedMessageToken;

///
/// # Description
///
/// Interrupt handler for IKC (inter-kernel communication) notifications.
///
/// The VMM injects this interrupt (IRQ 9) whenever a new message is enqueued for the guest. Its
/// sole purpose is to break the CPU out of `HLT` early so the kernel does not have to wait for the
/// next timer tick before it returns to its idle loop.
///
/// The handler is intentionally *ack-only*: it does not poll IKC messages here. Message polling
/// mutates scheduler state (it posts messages and wakes threads, moving processes between run
/// queues) and is only safe at the kernel's controlled scheduling points — the idle-loop poll and
/// the kcall trailing-poll — where the running process is the kernel idle context. IRQ 9, by
/// contrast, is delivered whenever interrupts are enabled, i.e. while a *user* process is the
/// running process; polling from that context corrupts run-queue bookkeeping and leaks a process
/// from the scheduler. Acknowledging the interrupt and letting the idle loop perform the poll
/// preserves the wake-from-`HLT` latency benefit without re-entering the scheduler from interrupt
/// context.
///
/// # Safety
///
/// Called from interrupt context with interrupts disabled on a single-core system. The handler
/// touches no kernel state, so it is safe to invoke regardless of what the interrupted code was
/// doing.
///
#[cfg(feature = "microvm")]
unsafe fn ikc_interrupt_handler(_intnum: InterruptNumber) {
    // Ack-only: the actual IKC polling happens at the kernel's controlled scheduling points (idle
    // loop and kcall trailing-poll). Do not re-enter the scheduler from interrupt context.
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
    // Retained pointer-typed entry point (used by callers outside `pm`, e.g. `ipc`/`io`). It only
    // carries the source address, so lower it to a `VirtualAddress` and delegate to the
    // address-typed helper. `src as usize` is a supported pointer-to-integer cast.
    copy_from_user_addr(pm, pid, dst, VirtualAddress::from_raw_value(src as usize))
}

///
/// # Description
///
/// Address-typed counterpart of [`copy_from_user`]. The user-space source is supplied as a
/// [`VirtualAddress`] — the type the copy path already lowers to internally — so callers that
/// already hold the address as an integer/`VirtualAddress` need not synthesize a raw pointer just
/// to satisfy the signature. Behavior is identical to [`copy_from_user`]: the source address is
/// never dereferenced as a kernel pointer; only its bits are used to copy through the target
/// process's page tables.
///
pub fn copy_from_user_addr<T>(
    pm: &mut ProcessManager,
    pid: ProcessIdentifier,
    dst: &mut T,
    src: VirtualAddress,
) -> Result<(), Error> {
    let dst: VirtualAddress = VirtualAddress::from_raw_value(dst as *mut T as usize);
    let size: usize = core::mem::size_of::<T>();

    pm.vmcopy_from_user(pid, dst, src, size)
}

pub fn copy_to_user<T>(
    pm: &mut ProcessManager,
    pid: ProcessIdentifier,
    dst: *mut T,
    src: &T,
) -> Result<(), Error> {
    // Retained pointer-typed entry point (used by callers outside `pm`, e.g. `ipc`/`io`). It only
    // carries the destination address, so lower it to a `VirtualAddress` and delegate to the
    // address-typed helper. `dst as usize` is a supported pointer-to-integer cast.
    copy_to_user_addr(pm, pid, VirtualAddress::from_raw_value(dst as usize), src)
}

///
/// # Description
///
/// Address-typed counterpart of [`copy_to_user`]. The user-space destination is supplied as a
/// [`VirtualAddress`] — the type the copy path already lowers to internally — so callers that
/// already hold the address as an integer/`VirtualAddress` need not synthesize a raw pointer just
/// to satisfy the signature. Behavior is identical to [`copy_to_user`]: the destination address is
/// never dereferenced as a kernel pointer; only its bits are used to copy through the target
/// process's page tables.
///
pub fn copy_to_user_addr<T>(
    pm: &mut ProcessManager,
    pid: ProcessIdentifier,
    dst: VirtualAddress,
    src: &T,
) -> Result<(), Error> {
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
    ProcessManager::init(interrupt_capable, kernel, root, tm)?;

    #[cfg(feature = "test")]
    {
        let passed: bool = test::test();
        assert!(passed, "pm in-kernel tests failed");
    }

    Ok(())
}
