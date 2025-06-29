// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::collections::LinkedList;
use ::sys::{
    ipc::Message,
    pm::ThreadIdentifier,
};

//==================================================================================================
//  Structures
//==================================================================================================

#[derive(Default)]
pub struct Mailbox {
    buffer: LinkedList<Message>,
}

//==================================================================================================
//  Implementations
//==================================================================================================

impl Mailbox {
    pub fn send(&mut self, message: Message) {
        self.buffer.push_back(message);
    }

    pub fn receive(&mut self, tid: ThreadIdentifier) -> Option<Message> {
        // Locate the first message that the given thread received.
        let message_index = self
            .buffer
            .iter()
            .position(|msg| { msg.destination }.as_id() == Err(tid));

        // If a message was found, remove it from the buffer and return it.
        if let Some(index) = message_index {
            return Some(self.buffer.remove(index));
        }

        // Locate the first message that the current thread received.
        let message_index = self
            .buffer
            .iter()
            .position(|msg| { msg.destination }.as_id().is_ok());

        // If a message was found, remove it from the buffer and return it.
        if let Some(index) = message_index {
            return Some(self.buffer.remove(index));
        }

        None
    }
}
