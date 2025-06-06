// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    event::EventManager,
    kcall::{
        KcallArgs,
        KcallResult,
    },
    pm::{
        self,
        ProcessManager,
        SleepError,
    },
};
use ::alloc::boxed::Box;
use ::sys::{
    error::Error,
    ipc::{
        Message,
        MessageSender,
        MessageType,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn do_send(pm: &mut ProcessManager, message: Box<Message>) -> Result<(), Error> {
    trace!("do_send(): src={:?}, dst={:?}", { message.source }, { message.destination });

    // TODO: Check if source process has permission to send message to destination process.

    // Post message.
    EventManager::post_message(pm, message.destination, message)
}

pub fn send(pm: &mut ProcessManager, args: &KcallArgs) -> KcallResult {
    let src_pid: ProcessIdentifier = args.pid;
    let src_tid: ThreadIdentifier = args.tid;

    // Copy message to kernel space.
    let mut message: Message = Message::default();
    if let Err(e) = pm::copy_from_user(pm, src_pid, &mut message, args.arg0 as *const Message) {
        return KcallResult::Error(e.code.into());
    }

    // Check if message source is invalid.
    if { message.source } != MessageSender::from(src_tid) && { message.source }
        != MessageSender::from(src_pid)
    {
        let reason: &str = "invalid message source";
        error!("do_send(): {reason:?} (message={:?})", message);
    }

    // Route message based on its type.
    match message.message_type {
        // Inter-kernel communication.
        MessageType::Ikc => {
            cfg_if::cfg_if! {
                // Check if standard input/output is available.
                if #[cfg(feature = "stdio")] {
                    // It is, so write message to standard output.
                    match crate::stdio::write(message) {
                        Ok(_) => KcallResult::ok(),
                        Err(e) => KcallResult::Error(e.code.into()),
                    }
                } else {
                    // Standard input/output is not available.
                    error!("send(): stdio is not available");
                    KcallResult::Error(sys::error::ErrorCode::ProtocolNotSupported.into())
                }
            }
        },
        // Local-host communication.
        _ => {
            // Post message.
            let message: Box<Message> = Box::new(message);
            match do_send(pm, message) {
                Ok(()) => KcallResult::ok(),
                Err(e) => KcallResult::Error(e.code.into()),
            }
        },
    }
}

pub unsafe fn recv(
    tid: ThreadIdentifier,
    pid: ProcessIdentifier,
    msg: usize,
) -> Result<(), SleepError> {
    if pid != ProcessIdentifier::INITD {
        trace!("do_recv(): pid={:?}", pid);
    }

    match EventManager::wait(tid, pid) {
        Ok(message) => {
            pm::copy_to_user(ProcessManager::get_mut(), pid, msg as *mut Message, &message)
                .map_err(SleepError::Generic)
        },
        Err(sleep_error) => Err(sleep_error),
    }
}
