// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ipc::IpcManager,
    kcall::KcallArgs,
    pm::{
        self,
        ProcessManager,
    },
};
use ::kcall::{
    Error,
    ErrorCode,
    Message,
    ProcessIdentifier,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn send(pm: &mut ProcessManager, args: &KcallArgs) -> i32 {
    let src: ProcessIdentifier = match ProcessManager::get_pid() {
        Ok(pid) => pid,
        Err(e) => return e.code.into_errno(),
    };

    // Copy message to kernel space.
    let mut message: Message = Message::default();
    if let Err(e) = pm::copy_from_user(src, &mut message, args.arg0 as *const Message) {
        return e.code.into_errno();
    }

    // Sanity check message source.
    if message.source != src {
        let reason: &str = "invalid message source";
        error!("send(): {}", reason);
        return Error::new(ErrorCode::InvalidArgument, reason)
            .code
            .into_errno();
    }

    // TODO: Check if source process has permission to send message to destination process.

    // Post message.
    match IpcManager::post_message(pm, src, message) {
        Ok(_) => 0,
        Err(e) => e.code.into_errno(),
    }
}

pub fn recv(msg: usize) -> i32 {
    let pid: ProcessIdentifier = match ProcessManager::get_pid() {
        Ok(pid) => pid,
        Err(e) => return e.code.into_errno(),
    };

    match IpcManager::receive_message(pid) {
        Ok(message) => {
            if let Err(e) = pm::copy_to_user(pid, msg as *mut Message, &message) {
                return e.code.into_errno();
            }
            0
        },
        Err(e) => e.code.into_errno(),
    }
}
