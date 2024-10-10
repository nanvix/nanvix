// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

use ::nvx::{
    ipc::Message,
    sys::error::Error,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[no_mangle]
pub fn main() -> Result<(), Error> {
    // Magic string that will case the server to exit.
    const MAGIC_STRING: &str = "exit";

    // Loop until we receive the magic string.
    loop {
        // Block until a message is received.
        match ::nvx::ipc::recv() {
            // Succeeded to receive a message.
            Ok(message) => {
                // Attempt interpret the payload as a string.
                if let Ok(payload) = core::str::from_utf8(&message.payload) {
                    // Check if we received the magic string.
                    if payload.trim_end_matches('\0').trim() == MAGIC_STRING {
                        break Ok(());
                    }
                }

                // Send a response.
                let response: Message = Message::new(
                    message.destination,
                    message.source,
                    message.message_type,
                    None,
                    message.payload,
                );

                // Send the response and check if we failed.
                if let Err(e) = ::nvx::ipc::send(&response) {
                    ::nvx::log!("failed to send message (error={:?}", e.code);
                }
            },
            // Failed to receive a message.
            Err(e) => {
                ::nvx::log!("failed to receive message (error={:?})", e);
            },
        }
    }
}
