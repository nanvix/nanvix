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
    pm::ProcessManager,
};
use ::sys::{
    event::ProcessTerminationInfo,
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
/// Kernel call handler.
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
            Ok(Some((pid, status))) => {
                // Check if the process manager daemon (PROCD) terminated.
                if pid == ProcessIdentifier::PROCD {
                    // It was, so we should shutdown.
                    break status;
                }
                // Record the termination so the main loop can publish a process-termination
                // scheduling event below. Buffering it (rather than notifying inline) lets the
                // same retry/backpressure as process creation apply, so termination events are not
                // silently lost when the scheduling-event queue is momentarily full.
                pm!().push_pending_termination(ProcessTerminationInfo::new(pid, status));
                harvested_process = true;
            },
            Err(e) => {
                error!("failed to harvest zombies: {:?}", e);
            },
        }

        // Publish process-creation scheduling events before process-termination events. A process
        // is always created before it terminates, so draining creations first biases delivery
        // toward a child's creation reaching `procd` ahead of its termination: when it does,
        // `procd` records the child's lineage from the creation event before acting on the
        // termination, keeping the termination off its `early_terminations` reconciliation path.
        // This is only a bias, not a guarantee: `EventManager::try_wait()` scans the creation and
        // termination sub-queues round-robin, so a termination can still be delivered ahead of a
        // queued creation (which `procd` reconciles). Each pending record is drained while no
        // reference to the process manager is held, so subscribers can be woken safely.
        let mut notified_creation: bool = false;
        while let Some(info) = pm!().take_pending_creation() {
            // SAFETY: the calling process does not hold a reference to the inner state of the
            // process manager.
            match unsafe { EventManager::notify_process_creation(info) } {
                Ok(()) => notified_creation = true,
                Err(e) => {
                    error!("failed to notify process creation: {:?}", e);
                    // Delivery failed without buffering the notification (the scheduling-event
                    // queue is full, or waking a subscriber failed and the entry was rolled back).
                    // Restore the record at the front of the queue so it is retried on a later
                    // iteration instead of being lost, and stop draining to avoid spinning on the
                    // same failure.
                    pm!().requeue_pending_creation(info);
                    break;
                },
            }
        }

        // Publish process-termination scheduling events for harvested processes after attempting to
        // publish any pending creations. This preserves creation-before-termination ordering when
        // both events can be delivered; under backpressure (e.g., the creation scheduling queue is
        // full), terminations may still be published and `procd` may need to reconcile them.
        let mut notified_termination: bool = false;
        while let Some(info) = pm!().take_pending_termination() {
            // SAFETY: the calling process does not hold a reference to the inner state of the
            // process manager.
            match unsafe { EventManager::notify_process_termination(info) } {
                Ok(()) => notified_termination = true,
                Err(e) => {
                    error!("failed to notify process termination: {:?}", e);
                    // Delivery failed without buffering the notification (the scheduling-event
                    // queue is full, or waking a subscriber failed and the entry was rolled back).
                    // Restore the record at the front of the queue so it is retried on a later
                    // iteration instead of being lost, and stop draining to avoid spinning on the
                    // same failure.
                    pm!().requeue_pending_termination(info);
                    break;
                },
            }
        }

        // No work to do, so yield the CPU.
        if !message_received && !harvested_process && !notified_termination && !notified_creation {
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

    while let Ok(Some((pid, status))) = pm!().harvest_zombies(mm!()) {
        info!("harvested zombie process: pid={:?}, status={:?}", pid, status);
    }

    status
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
