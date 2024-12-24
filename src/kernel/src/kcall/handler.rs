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
    kcall::ScoreBoard,
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
};

//==================================================================================================
//  Standalone Functions
//==================================================================================================
///
/// # Description
///
/// Kernel call handler.
///
pub fn kcall_handler(hal: &mut Hal, mm: &mut VirtMemoryManager, pm: &mut ProcessManager) {
    if let Err(e) = event::init(hal) {
        panic!("failed to initialize event manager: {:?}", e);
    }

    loop {
        // Read kernel call arguments from the scoreboard.
        match ScoreBoard::get_mut() {
            Ok(scoreboard) => match scoreboard.handle() {
                Ok(args) => {
                    let ret: i32 = match KcallNumber::from(args.number) {
                        KcallNumber::Debug => debug::debug(args),
                        KcallNumber::GetPid => {
                            // NOTE: this should be handled by the dispatcher.
                            // However we emit an invalid system call, just in case.
                            error!("cannot handle getpid()");
                            ErrorCode::InvalidSysCall.into_errno()
                        },
                        KcallNumber::GetTid => {
                            // NOTE: this should be handled by the dispatcher.
                            // However we emit an invalid system call, just in case.
                            error!("cannot handle gettid()");
                            ErrorCode::InvalidSysCall.into_errno()
                        },
                        KcallNumber::GetUid => pm::getuid(pm, args),
                        KcallNumber::GetGid => pm::getgid(pm, args),
                        KcallNumber::GetEuid => pm::geteuid(pm, args),
                        KcallNumber::GetEgid => pm::getegid(pm, args),
                        KcallNumber::SetUid => pm::setuid(pm, args),
                        KcallNumber::SetGid => pm::setgid(pm, args),
                        KcallNumber::SetEuid => pm::seteuid(pm, args),
                        KcallNumber::SetEgid => pm::setegid(pm, args),
                        KcallNumber::CapCtl => pm::capctl(pm, args),
                        KcallNumber::Terminate => pm::terminate(pm, args),
                        KcallNumber::EventCtrl => event::evctrl(pm, args),
                        KcallNumber::MemoryMap => pm::mmap(pm, mm, args),
                        KcallNumber::MemoryUnmap => pm::munmap(pm, mm, args),
                        KcallNumber::MemoryCtrl => pm::mctrl(pm, mm, args),
                        KcallNumber::MemoryCopy => pm::mcopy(mm, args),
                        KcallNumber::Send => ipc::send(pm, args),
                        KcallNumber::AllocMmio => io::mmio_alloc(hal, pm, args),
                        KcallNumber::FreeMmio => io::mmio_free(pm, args),
                        KcallNumber::AllocPmio => io::pmio_alloc(hal, pm, args),
                        KcallNumber::FreePmio => io::pmio_free(pm, args),
                        KcallNumber::ReadPmio => io::pmio_read(pm, args),
                        KcallNumber::WritePmio => io::pmio_write(pm, args),
                        _ => {
                            error!("invalid kernel call");
                            ErrorCode::InvalidSysCall.into_errno()
                        },
                    };
                    if let Err(e) = scoreboard.handled(ret) {
                        warn!("failed to signal kernel call handled: {:?}", e)
                    }
                },
                Err(e) => match e.code {
                    ErrorCode::Interrupted => break,
                    ErrorCode::OperationWouldBlock => {
                        if let Err(e) = ProcessManager::switch() {
                            error!("context switch failed: {:?}", e);
                        }
                    },
                    _ => {
                        error!("failed to handle kernel call: {:?}", e);
                    },
                },
            },
            Err(e) => {
                warn!("failed to get scoreboard: {:?}", e)
            },
        };

        cfg_if::cfg_if! {
            if #[cfg(feature = "stdio")] {
                // Check if the number of buffered messages in the kernel is not to high. We don't
                // want to keep pushing messages to the kernel and then run out of memory.
                if let Ok(number_buffered_messages) = pm.number_buffered_messages()  {
                    if number_buffered_messages < ::sys::config::kernel::MAX_IKC_MESSAGES {
                        // The number of messages that are buffered in the kernel is not too high,
                        // So attempt to read an inter-kernel communication message from the
                        // kernel's standard input.
                        match crate::stdio::read() {
                            // No message is available.
                            Ok(None) => {},
                            // A message is available.
                            Ok(Some(message)) => {
                                if let Err(e) = EventManager::post_message(pm, message.destination, message) {
                                    warn!("failed to post message (error={:?})", e);
                                }
                            }
                            // Failed to read message.
                            Err(e) => {
                                warn!("failed to read message (error={:?})", e);
                            }
                        }
                    }
                }
            }
        }

        match pm.harvest_zombies() {
            Ok(None) => {},
            Ok(Some((pid, status))) => {
                // Check if init daemon process terminated.
                if pid == ProcessIdentifier::INITD {
                    // It was, so we should shutdown.
                    break;
                }
                match EventManager::notify_process_termination(ProcessTerminationInfo::new(
                    pid, status,
                )) {
                    Ok(()) => {},
                    Err(e) => {
                        error!("failed to notify process termination: {:?}", e);
                    },
                }
            },
            Err(e) => {
                error!("failed to harvest zombies: {:?}", e);
            },
        }
    }

    while let Ok(Some((pid, status))) = pm.harvest_zombies() {
        info!("harvested zombie process: pid={:?}, status={:?}", pid, status);
    }
}
