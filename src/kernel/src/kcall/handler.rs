// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use crate::{
    debug,
    event::{
        self,
        EventManager,
    },
    hal::Hal,
    io,
    ipc,
    kcall::{
        KcallResult,
        ScoreBoard,
    },
    mm::VirtMemoryManager,
    pm::{
        self,
        ProcessManager,
    },
};
use ::sys::{
    error::ErrorCode,
    event::ProcessTerminationInfo,
    number::KcallNumber,
    pm::ProcessIdentifier,
    ExitStatus,
};

#[cfg(feature = "stdio")]
use ::config::kernel::IKC_POLL_BATCH_SIZE;

//==================================================================================================
//  Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Kernel call handler.
///
pub fn kcall_handler(
    hal: &mut Hal,
    mm: &mut VirtMemoryManager,
    pm: &mut ProcessManager,
) -> ExitStatus {
    if let Err(e) = event::init(hal) {
        panic!("failed to initialize event manager: {:?}", e);
    }

    let status: ExitStatus = loop {
        // Attempt to handle a kernel call.
        let mut kcall_handled: bool = false;
        match unsafe { ScoreBoard::handle() } {
            Ok((slot_index, args)) => {
                let ret: KcallResult = match KcallNumber::from(args.number) {
                    KcallNumber::Debug => debug::debug(pm, args),
                    KcallNumber::GetPid => {
                        // NOTE: this should be handled by the dispatcher.
                        // However we emit an invalid system call, just in case.
                        error!("cannot handle getpid()");
                        KcallResult::Error(ErrorCode::InvalidSysCall.into())
                    },
                    KcallNumber::GetTid => {
                        // NOTE: this should be handled by the dispatcher.
                        // However we emit an invalid system call, just in case.
                        error!("cannot handle gettid()");
                        KcallResult::Error(ErrorCode::InvalidSysCall.into())
                    },
                    KcallNumber::CapCtl => pm::capctl(pm, args),
                    KcallNumber::Terminate => pm::terminate(pm, args),
                    KcallNumber::EventCtrl => event::evctrl(pm, args),
                    KcallNumber::MemoryMap => pm::mmap(pm, mm, args),
                    KcallNumber::MemoryUnmap => pm::munmap(pm, mm, args),
                    KcallNumber::MemoryCtrl => pm::mctrl(pm, mm, args),
                    KcallNumber::MemoryCopy => pm::mcopy(pm, mm, args),
                    KcallNumber::Send => ipc::send(pm, args),
                    KcallNumber::AllocMmio => io::mmio_alloc(hal, pm, args),
                    KcallNumber::FreeMmio => io::mmio_free(pm, args),
                    KcallNumber::AllocPmio => io::pmio_alloc(hal, pm, args),
                    KcallNumber::FreePmio => io::pmio_free(pm, args),
                    KcallNumber::ReadPmio => io::pmio_read(pm, args),
                    KcallNumber::WritePmio => io::pmio_write(pm, args),
                    KcallNumber::GetTime => pm::gettime(pm, args),
                    KcallNumber::CreateThread => pm::create_thread(pm, mm, args),
                    KcallNumber::SetThreadDataArea => pm::set_thread_data_area(pm, args),
                    KcallNumber::GetThreadDataArea => pm::get_thread_data_area(pm, args),
                    _ => {
                        error!("invalid kernel call");
                        KcallResult::Error(ErrorCode::InvalidSysCall.into())
                    },
                };

                // SAFETY: the calling process does not hold a reference to the inner state of the process manager.
                if let Err(e) = unsafe { ScoreBoard::handled(slot_index, ret) } {
                    warn!("failed to signal kernel call handled: {:?}", e)
                }

                kcall_handled = true;
            },
            Err(error) => match error.code {
                ErrorCode::TryAgain => {},
                _ => {
                    // This condition should never happen because the only error that should
                    // happen for `ScoreBoard::handle()` is `ErrorCode::TryAgain`.
                    unreachable!("failed to handle kernel call (error={:?})", error);
                },
            },
        };

        // Check if inter-kernel communication messages are available.
        cfg_if::cfg_if! {
            if #[cfg(feature = "stdio")] {
                let mut message_received: bool = false;
                for _ in 0..IKC_POLL_BATCH_SIZE {
                    // Check if the number of buffered messages in the kernel is not too high. We don't
                    // want to keep pushing messages to the kernel and then run out of memory.
                    if let Ok(number_buffered_messages) = pm.number_buffered_messages() {
                        if number_buffered_messages < config::kernel::MAX_IKC_MESSAGES {
                            // The number of messages that are buffered in the kernel is not too high,
                            // So attempt to read an inter-kernel communication message from the
                            // kernel's standard input.
                            match crate::stdio::read() {
                                // No message is available.
                                Ok(None) => break,
                                // A message is available.
                                Ok(Some(message)) => {
                                    if let Err(e) = EventManager::post_message(pm, message.destination, message) {
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
                    }
                }
            } else {
                // No inter-kernel communication messages are available.
                let message_received: bool = false;
            }
        }

        // Attempt to harvest zombie processes.
        let mut harvested_process: bool = false;
        match pm.harvest_zombies(mm) {
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
        if !kcall_handled && !message_received && !harvested_process {
            // SAFETY: the kernel process does not hold any resources.
            if let Err(error) = unsafe { ProcessManager::giveup() } {
                error!("context switch failed (error={:?})", error);
            }
        }
    };

    while let Ok(Some((pid, status))) = pm.harvest_zombies(mm) {
        info!("harvested zombie process: pid={:?}, status={:?}", pid, status);
    }

    status
}
