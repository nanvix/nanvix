// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    event::EventManager,
    kcall::KcallResult,
    pm::{
        self,
        ProcessManager,
    },
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
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

fn do_send(pm: &mut ProcessManager, message: Message) -> Result<(), Error> {
    trace!("src={:?}, dst={:?}", { message.source }, { message.destination });

    // TODO: Check if source process has permission to send message to destination process.

    // Post message.
    EventManager::post_message(pm, message.destination, message)
}

///
/// # Description
///
/// Kernel call handler for sending an inter-process message.
///
/// # Parameters
///
/// - `pid`: Identifier of the calling process.
/// - `tid`: Identifier of the calling thread.
/// - `arg0`: User-space pointer to the [`Message`] to send.
///
/// # Returns
///
/// A [`KcallResult`] indicating success or the error code.
///
pub fn send(pid: ProcessIdentifier, tid: ThreadIdentifier, arg0: u32) -> KcallResult {
    // SAFETY: the process manager is initialized and access is synchronized.
    let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };

    let src_pid: ProcessIdentifier = pid;
    let src_tid: ThreadIdentifier = tid;

    // Copy message to kernel space.
    let mut message: Message = Message::default();
    if let Err(e) = pm::copy_from_user(pm, src_pid, &mut message, arg0 as *const Message) {
        return KcallResult::Error(e.code.into());
    }

    // Reject messages whose source does not match the calling thread or process. Without this
    // check a process could forge the `source` field to impersonate another (possibly privileged)
    // peer, since receivers authenticate callers on `message.source`.
    if { message.source } != MessageSender::from(src_tid) && { message.source }
        != MessageSender::from(src_pid)
    {
        error!("invalid message source (message={message:?})");
        return KcallResult::Error(ErrorCode::OperationNotPermitted.into());
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
                    error!("stdio is not available");
                    KcallResult::Error(sys::error::ErrorCode::ProtocolNotSupported.into())
                }
            }
        },
        // Local-host communication.
        _ => {
            // Post message.
            match do_send(pm, message) {
                Ok(()) => KcallResult::ok(),
                Err(e) => KcallResult::Error(e.code.into()),
            }
        },
    }
}
