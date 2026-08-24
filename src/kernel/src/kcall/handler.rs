// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    event::{
        self,
        EventManager,
    },
    mm::VirtMemoryManager,
    pm::{
        ProcessManager,
        SignalDeliveryOutcome,
    },
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    pm::ProcessIdentifier,
    ExitStatus,
};

#[cfg(feature = "stdio")]
use ::config::kernel::IKC_POLL_BATCH_SIZE;

//==================================================================================================
//  Standalone Functions
//==================================================================================================

// Convenience macro to obtain a mutable reference to the process manager.
//
// # Safety
//
// Each expansion produces a fresh `&mut ProcessManager` via `get_mut()`. Callers must ensure
// that borrows from separate expansions do not overlap (i.e., drop the reference before the
// next `pm!()` invocation).
macro_rules! pm {
    () => {
        // SAFETY: the process manager is initialized, this is a single-core system,
        // and the kernel runs with interrupts disabled.
        unsafe { ProcessManager::get_mut() }
    };
}

// Convenience macro to obtain a mutable reference to the virtual memory manager.
//
// # Safety
//
// Each expansion produces a fresh `&mut VirtMemoryManager` via `get_mut()`. Callers must ensure
// that borrows from separate expansions do not overlap (i.e., drop the reference before the
// next `mm!()` invocation).
macro_rules! mm {
    () => {
        // SAFETY: the memory manager is initialized, this is a single-core system,
        // and the kernel runs with interrupts disabled.
        unsafe { VirtMemoryManager::get_mut() }
    };
}

///
/// # Description
///
/// Drains one deferred lifecycle-owner wakeup request. A failed notification is re-armed while the
/// lifecycle record remains buffered, so a later kernel-loop iteration retries it.
///
/// # Parameters
///
/// - `notify`: Operation that wakes the current lifecycle owner.
///
/// # Returns
///
/// `true` if a pending request was successfully delivered, otherwise `false`.
///
pub(crate) fn drain_lifecycle_wakeup<F>(notify: F) -> bool
where
    F: FnOnce() -> Result<(), Error>,
{
    if !pm!().take_lifecycle_wakeup_request() {
        return false;
    }

    match notify() {
        Ok(()) => true,
        Err(error) => {
            error!("failed to wake lifecycle owner: {error:?}");
            pm!().request_lifecycle_wakeup();
            false
        },
    }
}

///
/// # Description
///
/// Kernel call handler.
///
/// # Returns
///
/// The exit status that should be reported when the kernel shuts down.
///
/// # Panics
///
/// This function panics if the event manager cannot be initialized, lifecycle state was already
/// disposed, or lifecycle reservation accounting is inconsistent during shutdown.
///
pub fn kcall_handler() -> ExitStatus {
    if let Err(e) = event::init() {
        panic!("failed to initialize event manager: {:?}", e);
    }

    // Signal the VMM that kernel startup is complete and user-space is about to start.
    crate::hal::platform::signal_startup_complete();

    let status: ExitStatus = loop {
        // Check if inter-kernel communication messages are available.
        let message_received: bool = poll_ikc_messages();

        // Attempt to harvest zombie processes.
        let mut harvested_process: bool = false;
        match pm!().harvest_zombies(mm!()) {
            Ok(None) => {},
            Ok(Some(termination)) => {
                // Check if the process manager daemon (PROCD) terminated.
                if termination.info().pid == ProcessIdentifier::PROCD {
                    // It was, so we should shutdown.
                    break termination.into_info().status;
                }
                harvested_process = true;
            },
            Err(e) => {
                error!("failed to harvest zombies: {:?}", e);
            },
        }

        // Lifecycle records are already committed to the ordered delivery broker. Only owner
        // wakeup is deferred so the event manager does not re-enter the process manager while it is
        // borrowed.
        // SAFETY: the calling process does not hold a reference to the inner state of the process
        // manager.
        let notified_lifecycle: bool =
            drain_lifecycle_wakeup(|| unsafe { EventManager::notify_process_lifecycle() });

        // No work to do, so yield the CPU.
        if !message_received && !harvested_process && !notified_lifecycle {
            // Flush the kernel log buffer.
            // SAFETY: the standard output device is present, initialized, and accessed
            // exclusively from a single core with interrupts disabled.
            unsafe { crate::klog::flush() };

            // SAFETY: the kernel process does not hold any resources.
            if let Err(error) = unsafe { ProcessManager::giveup() } {
                error!("context switch failed (error={:?})", error);
            }
        }
    };

    while let Ok(Some(termination)) = pm!().harvest_zombies(mm!()) {
        let info = termination.into_info();
        info!("harvested zombie process: pid={:?}, status={:?}", info.pid, info.status);
    }
    pm!().dispose_lifecycle();

    status
}

///
/// # Description
///
/// Delivers a pending caught signal to the running thread at the kernel-call return-to-user
/// boundary, redirecting it through its user-space handler.
///
/// This is the asynchronous-delivery checkpoint: it runs at the end of every kernel call, mirroring
/// [`poll_ikc_messages`]. When a signal frame cannot be built safely, the offending process is
/// terminated via its default action.
///
/// # Parameters
///
/// - `result`: The return value the interrupted kernel call would otherwise deliver to user space.
///
pub fn deliver_pending_signals(result: i64) {
    match pm!().try_deliver_signal(result) {
        SignalDeliveryOutcome::None | SignalDeliveryOutcome::Delivered => {},
        SignalDeliveryOutcome::Escalate => {
            // The frame could not be built safely (for example, a corrupt user stack). Take the
            // signal's default action and terminate the process. On success `exit()` switches
            // context and never returns.
            // SAFETY: the calling process is not the kernel and no borrow of the process manager is
            // held at this point.
            match unsafe { ProcessManager::exit(ExitStatus::from(ErrorCode::Interrupted)) } {
                Ok(never) => never,
                Err(error) => {
                    error!("failed to terminate after unsafe signal frame (error={error:?})");
                },
            }
        },
    }
}

///
/// # Description
///
/// Polls inter-kernel communication messages from the kernel's standard input and dispatches them
/// to the appropriate destination process.
///
/// # Returns
///
/// `true` if at least one message was received, `false` otherwise.
///
pub fn poll_ikc_messages() -> bool {
    cfg_if::cfg_if! {
        if #[cfg(feature = "stdio")] {
            let mut message_received: bool = false;
            for _ in 0..IKC_POLL_BATCH_SIZE {
                // Check if the number of buffered messages in the kernel is too high. We don't want
                // to keep pushing messages to the kernel and then run out of memory.
                let number_buffered_messages: usize = pm!().number_buffered_messages();
                if number_buffered_messages >= config::kernel::MAX_IKC_MESSAGES {
                    break;
                }

                // The number of messages that are buffered in the kernel is not too high,
                // So attempt to read an inter-kernel communication message from the
                // kernel's standard input.
                match crate::stdio::read() {
                    // No more messages are available.
                    Ok(None) => break,
                    // A message is available.
                    Ok(Some(message)) => {
                        if let Err(e) = EventManager::post_message(pm!(), message.destination, message) {
                            warn!("failed to post message (error={:?})", e);
                        }
                        message_received = true;
                    }
                    // Failed to read message.
                    Err(e) => {
                        warn!("failed to read message (error={:?})", e);
                    }
                }
            }
            message_received
        } else {
            // No inter-kernel communication messages are available.
            false
        }
    }
}
