// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::platform;
use ::core::mem;
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
    trace!(
        "write(): message.type={:?}, message.source={:?}, message.destination={:?}",
        { message.message_type },
        { message.source },
        { message.destination }
    );

    // Checks if message type is not supported.
    if { message.message_type } != MessageType::Ikc {
        let reason: &str = "unsupported message type";
        error!("write(): {}", reason);
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    let bytes: [u8; mem::size_of::<Message>()] = message.to_bytes();

    // Write message to the kernel's standard output.
    // SAFETY: The standard output is present, initialized and thread-safe to write.
    unsafe {
        // NOTE: we assume that page is tagged as writethrough-enabled and cache-disabled.
        platform::vmbus_write(&bytes as *const u8);
    }

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

    // Read message from the kernel's standard input.
    // SAFETY: The standard input is present, initialized and thread-safe to read.
    unsafe {
        // NOTE: we assume that page is tagged as writethrough-enabled and cache-disabled.
        platform::vmbus_read(&mut message as *mut u8);
    };

    // Convert message to Message struct.
    match Message::try_from_bytes(message) {
        Ok(message) => {
            // Check if message is empty.
            if { message.message_type } == MessageType::Empty {
                Ok(None)
            } else {
                // NOTE: trace command after reading the first byte, to avoid flooding the log.
                trace!("read()");
                Ok(Some(message))
            }
        },
        // No message available.
        Err(e) if e.code == ErrorCode::NoMessageAvailable => Ok(None),
        Err(e) => {
            warn!("read(): {:?} ", e);
            Err(e)
        },
    }
}
