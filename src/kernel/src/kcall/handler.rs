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
    error::ErrorCode,
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
            Ok(Some(info)) => {
                // Check if the process manager daemon (PROCD) terminated.
                if info.pid == ProcessIdentifier::PROCD {
                    // It was, so we should shutdown.
                    break info.status;
                }
                // Record the termination so the main loop can publish a process-termination
                // scheduling event below. Buffering it (rather than notifying inline) lets the
                // same retry/backpressure as process creation apply, so termination events are not
                // silently lost when the scheduling-event queue is momentarily full.
                pm!().push_pending_termination(info);
                harvested_process = true;
            },
            Err(e) => {
                error!("failed to harvest zombies: {:?}", e);
            },
        }

        // Publish process-creation scheduling events before process-termination events. A process
        // is always created before it terminates, so draining creations first enqueues a child's
        // creation ahead of its termination in the event manager's single FIFO scheduling-event
        // queue. Because that queue is delivered in strict FIFO order, `procd` is guaranteed to
        // observe a child's creation before its termination, so it records the child's lineage from
        // the creation event before acting on the termination — no out-of-order reconciliation is
        // needed. Each pending record is drained while no reference to the process manager is held,
        // so subscribers can be woken safely.
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
        // publish any pending creations. Draining terminations after creations enqueues each
        // termination behind the matching creation in the event manager's single FIFO
        // scheduling-event queue, so `procd` always observes a creation before the matching
        // termination. The ordering also holds under backpressure: the shared queue fills while
        // draining creations above, so a termination whose creation could not be enqueued simply
        // fails to enqueue too and is requeued for a later iteration.
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

    while let Ok(Some(info)) = pm!().harvest_zombies(mm!()) {
        info!("harvested zombie process: pid={:?}, status={:?}", info.pid, info.status);
    }

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
