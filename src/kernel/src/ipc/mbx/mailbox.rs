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

///
/// # Description
///
/// Mailbox.
///
#[derive(Default)]
pub struct Mailbox {
    /// Buffered messages.
    buffer: LinkedList<Message>,
}

//==================================================================================================
//  Implementations
//==================================================================================================

impl Mailbox {
    ///
    /// # Description
    ///
    /// Posts a message into the mailbox.
    ///
    /// # Parameters
    ///
    /// - `message`: Message to be sent.
    ///
    pub fn send(&mut self, message: Message) {
        self.buffer.push_back(message);
    }

    ///
    /// # Description
    ///
    /// Checks whether the mailbox has no buffered messages.
    ///
    /// # Returns
    ///
    /// `true` if no messages are buffered in the mailbox, otherwise `false`.
    ///
    pub(crate) fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    ///
    /// # Description
    ///
    /// Attempts to consume a message addressed to the given thread or its process.
    ///
    /// # Parameters
    ///
    /// - `tid`: Target thread identifier.
    ///
    /// # Returns
    ///
    /// If a message that was addressed to the given thread or its process was found,
    /// it is returned. Otherwise, no message is returned instead.
    ///
    pub fn receive(&mut self, tid: ThreadIdentifier) -> Option<Message> {
        // Search for a message that is addressed to the thread.
        let message_index = self
            .buffer
            .iter()
            .position(|msg| { msg.destination }.tid == tid);

        // If a message was found, remove it from the buffer and return it.
        if let Some(index) = message_index {
            return Some(self.buffer.remove(index));
        }

        // Locate the first message that is addressed to the process.
        let message_index = self
            .buffer
            .iter()
            .position(|msg| { msg.destination }.tid.is_none());

        // If a message was found, remove it from the buffer and return it.
        if let Some(index) = message_index {
            return Some(self.buffer.remove(index));
        }

        None
    }

    ///
    /// # Description
    ///
    /// Removes every buffered message addressed exactly to a thread.
    ///
    /// # Parameters
    ///
    /// - `tid`: Identifier of the thread whose messages should be removed.
    ///
    /// # Returns
    ///
    /// The number of messages removed from the mailbox.
    ///
    pub fn purge_thread(&mut self, tid: ThreadIdentifier) -> usize {
        let mut retained: LinkedList<Message> = LinkedList::new();
        let mut removed: usize = 0;
        while let Some(message) = self.buffer.pop_front() {
            if { message.destination }.tid == tid {
                removed += 1;
            } else {
                retained.push_back(message);
            }
        }
        self.buffer = retained;
        removed
    }

    ///
    /// # Description
    ///
    /// Removes every buffered message.
    ///
    /// # Returns
    ///
    /// The number of messages removed from the mailbox.
    ///
    pub fn purge_all(&mut self) -> usize {
        let removed: usize = self.buffer.len();
        self.buffer.clear();
        removed
    }
}
