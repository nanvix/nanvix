// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(not(feature = "hyperlight"))]
use crate::event;
use crate::{
    event::EventManager,
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
#[cfg(not(feature = "hyperlight"))]
pub fn kcall_handler() -> ExitStatus {
    if let Err(e) = event::init() {
        panic!("failed to initialize event manager: {:?}", e);
    }

    // Signal the VMM that kernel startup is complete and user-space is about to start.
    crate::hal::platform::signal_startup_complete();

    kcall_event_loop()
}

///
/// # Description
///
/// Kernel event loop. Polls IKC messages, harvests zombie processes,
/// and yields the CPU when idle. Returns the exit status when the
/// init daemon terminates.
///
pub fn kcall_event_loop() -> ExitStatus {
    let status: ExitStatus = loop {
        // Check if inter-kernel communication messages are available.
        let message_received: bool = poll_ikc_messages();

        // Attempt to harvest zombie processes.
        let mut harvested_process: bool = false;
        match pm!().harvest_zombies(mm!()) {
            Ok(None) => {},
            Ok(Some((pid, status))) => {
                // Check if init daemon process terminated.
                if pid == ProcessIdentifier::INITD {
                    // It was, so we should shutdown.
                    break status;
                }
                // SAFETY: the calling process does not hold a reference to the inner state of the process manager.
                match unsafe {
                    EventManager::notify_process_termination(ProcessTerminationInfo::new(
                        pid, status,
                    ))
                } {
                    Ok(()) => harvested_process = true,
                    Err(e) => {
                        error!("failed to notify process termination: {:?}", e);
                    },
                }
            },
            Err(e) => {
                error!("failed to harvest zombies: {:?}", e);
            },
        }

        // No work to do, so yield the CPU.
        if !message_received && !harvested_process {
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
