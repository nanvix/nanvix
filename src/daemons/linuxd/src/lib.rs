// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

extern crate alloc;

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::{
    error::ErrorCode,
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    pm::ThreadIdentifier,
};

//==================================================================================================
// Exports
//==================================================================================================

pub mod args;
pub mod config;
pub mod dirent;
pub mod error;
pub mod fcntl;
pub mod linuxd;
pub mod message;
pub mod poll;
pub mod socket;
pub mod sys_select;
pub mod time;
pub mod times;
pub mod unistd;
pub mod user_vm_handle;
pub mod venv;
pub mod worker_thread;
pub use linuxd::LinuxDaemon;

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Maximum number of messages that can be queued in a channel to a worker thread.
///
pub const WORKER_THREAD_CHANNEL_CAPACITY: usize = 1024;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Builds an error response message.
///
/// # Parameters
///
/// - `tid`: Thread identifier.
/// - `error`: Error code.
///
/// # Returns
///
/// A message with the error response.
///
pub fn build_error(tid: ThreadIdentifier, error: ErrorCode) -> Message {
    Message::new(
        MessageSender::from(::syscall::LINUXD),
        MessageReceiver::from(tid),
        MessageType::Ikc,
        Some(error),
        [0u8; Message::PAYLOAD_SIZE],
    )
}
