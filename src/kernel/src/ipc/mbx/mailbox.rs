// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::collections::vec_deque::VecDeque;
use ::sys::{
    ipc::Message,
    pm::ThreadIdentifier,
};
use alloc::boxed::Box;

//==================================================================================================
//  Structures
//==================================================================================================

pub struct Mailbox {
    buffer: VecDeque<Box<Message>>,
}

//==================================================================================================
//  Implementations
//==================================================================================================

impl Mailbox {
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::with_capacity(1),
        }
    }

    pub fn send(&mut self, message: Box<Message>) {
        self.buffer.push_back(message);
    }

    pub fn receive(&mut self, tid: ThreadIdentifier) -> Option<Box<Message>> {
        // Locate the first message that the given thread received.
        let message_index = self
            .buffer
            .iter()
            .position(|msg| { msg.destination }.as_id() == Err(tid));

        // If a message was found, remove it from the buffer and return it.
        if let Some(index) = message_index {
            return self.buffer.remove(index);
        }

        // Locate the first message that the current thread received.
        let message_index = self
            .buffer
            .iter()
            .position(|msg| { msg.destination }.as_id().is_ok());

        // If a message was found, remove it from the buffer and return it.
        if let Some(index) = message_index {
            return self.buffer.remove(index);
        }

        None
    }
}
