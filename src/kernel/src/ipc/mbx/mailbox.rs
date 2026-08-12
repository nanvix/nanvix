// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::delivery_sequence::DeliverySequence;
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
    buffer: LinkedList<(DeliverySequence, Message)>,
}

//==================================================================================================
//  Implementations
//==================================================================================================

impl Mailbox {
    ///
    /// # Description
    ///
    /// Finds the oldest message eligible for the given thread, that is, a message addressed
    /// either to the thread itself or to its process.
    ///
    /// # Parameters
    ///
    /// - `tid`: Target thread identifier.
    ///
    /// # Returns
    ///
    /// If an eligible message was found, its index and sequence number are returned.
    /// Otherwise, nothing is returned instead.
    ///
    fn oldest_eligible(&self, tid: ThreadIdentifier) -> Option<(usize, DeliverySequence)> {
        self.buffer
            .iter()
            .enumerate()
            .filter(|(_, (_, message))| {
                let destination_tid: ThreadIdentifier = message.destination.tid;
                destination_tid == tid || destination_tid.is_none()
            })
            .min_by_key(|(_, (sequence, _))| *sequence)
            .map(|(index, (sequence, _))| (index, *sequence))
    }

    ///
    /// # Description
    ///
    /// Posts a message into the mailbox.
    ///
    /// # Parameters
    ///
    /// - `sequence`: Sequence number assigned to the message.
    /// - `message`: Message to be sent.
    ///
    pub fn send(&mut self, sequence: DeliverySequence, message: Message) {
        self.buffer.push_back((sequence, message));
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
    pub fn receive(&mut self, tid: ThreadIdentifier) -> Option<(DeliverySequence, Message)> {
        self.oldest_eligible(tid)
            .map(|(index, _)| self.buffer.remove(index))
    }

    ///
    /// # Description
    ///
    /// Peeks the sequence number of the oldest message eligible for the given thread.
    ///
    /// # Parameters
    ///
    /// - `tid`: Target thread identifier.
    ///
    /// # Returns
    ///
    /// If an eligible message was found, its sequence number is returned. Otherwise,
    /// nothing is returned instead.
    ///
    pub fn peek_sequence(&self, tid: ThreadIdentifier) -> Option<DeliverySequence> {
        self.oldest_eligible(tid).map(|(_, sequence)| sequence)
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
        let mut retained: LinkedList<(DeliverySequence, Message)> = LinkedList::new();
        let mut removed: usize = 0;
        while let Some((sequence, message)) = self.buffer.pop_front() {
            if { message.destination }.tid == tid {
                removed += 1;
            } else {
                retained.push_back((sequence, message));
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
