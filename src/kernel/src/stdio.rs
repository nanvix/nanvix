// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::platform,
    PERF_IKC_MESSAGES_RECEIVED,
    PERF_IKC_MESSAGES_SENT,
};
use ::core::{
    mem,
    sync::atomic::Ordering,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
        MessageType,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes an inter-kernel communication message to the kernel's standard output.
///
/// # Parameters
///
/// - `message`: Message to write.
///
/// # Returns
///
/// Upon success, empty is returned. Upon failure, an error is returned instead.
///
pub fn write(message: Message) -> Result<(), Error> {
    // Checks if message type is not supported.
    if { message.message_type } != MessageType::Ikc {
        let reason: &str = "unsupported message type";
        error!("{reason}");
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    let bytes: [u8; mem::size_of::<Message>()] = message.to_bytes();

    // Write message to the kernel's standard output.
    // SAFETY: The standard output is present, initialized and thread-safe to write.
    unsafe {
        // NOTE: we assume that page is tagged as writethrough-enabled and cache-disabled.
        platform::vmbus_write(&bytes as *const u8);
    }

    PERF_IKC_MESSAGES_SENT.fetch_add(1, Ordering::Relaxed);

    Ok(())
}

///
/// # Description
///
/// Reads an inter-kernel communication message from the kernel's standard input.
///
/// # Returns
///
/// Upon success, this function either returns the message read or `None` if there are no more
/// messages.  Upon failure, an error is returned instead.
///
pub fn read() -> Result<Option<Message>, Error> {
    const NBYTES: usize = core::mem::size_of::<Message>();
    let mut message: [u8; NBYTES] = [0; NBYTES];

    cfg_if::cfg_if! {
        if #[cfg(feature = "microvm")] {
            // Read credits register.
            let credits: u32 = unsafe {
                core::ptr::read_volatile(::config::microvm::DEFAULT_MICROVM_CTRL_CREDITS as *const u32)
            };
        }
        else if #[cfg(feature = "hyperlight")] {
            // Read credits register.
            let credits: u64 = unsafe {
                crate::hal::platform::hyperlight::peb::ProcessEnvironmentBlock::get_credits()?
            };
        }
    }

    // No message available.
    if credits == 0 {
        return Ok(None);
    }

    // Read message from the kernel's standard input.
    // SAFETY: The standard input is present, initialized and thread-safe to read.
    unsafe {
        // NOTE: we assume that page is tagged as writethrough-enabled and cache-disabled.
        platform::vmbus_read(&mut message as *mut u8);
    };

    PERF_IKC_MESSAGES_RECEIVED.fetch_add(1, Ordering::Relaxed);

    // Convert message to Message struct.
    match Message::try_from_bytes(message) {
        Ok(message) => Ok(Some(message)),
        // No message available.
        Err(e) if e.code == ErrorCode::NoMessageAvailable => Ok(None),
        Err(e) => {
            warn!("{e:?}");
            Err(e)
        },
    }
}
