// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::DeliverySequence;
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
    /// Reports whether a delivery sequence still identifies the oldest mailbox message eligible
    /// for a thread. This exposes the mailbox token invariant to in-kernel tests.
    ///
    /// # Parameters
    ///
    /// - `tid`: Identifier of the receiving thread.
    /// - `sequence`: Delivery sequence captured by the token under test.
    ///
    /// # Returns
    ///
    /// `true` if `sequence` identifies the current eligible message, otherwise `false`.
    ///
    #[cfg(feature = "test")]
    pub(crate) fn test_token_is_current(
        &self,
        tid: ThreadIdentifier,
        sequence: DeliverySequence,
    ) -> bool {
        matches!(
            self.oldest_eligible(tid),
            Some((_, selected_sequence)) if selected_sequence == sequence
        )
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
    /// Peeks the oldest message eligible for the given thread without consuming it.
    ///
    /// # Parameters
    ///
    /// - `tid`: Target thread identifier.
    ///
    /// # Returns
    ///
    /// The eligible message and its sequence number, or [`None`] if no message is eligible.
    ///
    pub fn peek(&self, tid: ThreadIdentifier) -> Option<(DeliverySequence, Message)> {
        let (index, sequence): (usize, DeliverySequence) = self.oldest_eligible(tid)?;
        self.buffer
            .iter()
            .nth(index)
            .map(|(_, message)| (sequence, message.clone()))
    }

    ///
    /// # Description
    ///
    /// Commits delivery of a previously peeked message.
    ///
    /// # Parameters
    ///
    /// - `tid`: Target thread identifier.
    /// - `sequence`: Sequence number returned by [`Self::peek`].
    ///
    /// # Returns
    ///
    /// `true` if the selected message was removed, or `false` if no eligible message exists.
    ///
    /// # Panics
    ///
    /// This function panics if `sequence` does not identify the oldest eligible message for
    /// `tid`. This indicates a stale, duplicate, or otherwise invalid delivery token.
    ///
    pub fn commit(&mut self, tid: ThreadIdentifier, sequence: DeliverySequence) -> bool {
        let Some((index, selected_sequence)) = self.oldest_eligible(tid) else {
            return false;
        };
        assert!(selected_sequence == sequence, "stale mailbox delivery token");
        self.buffer.remove(index);
        true
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
