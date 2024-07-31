// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
//  Imports
//==================================================================================================

use crate::pm::ProcessIdentifier;

//==================================================================================================
//  Structures
//==================================================================================================

pub enum MessageType {
    Interrupt,
    Exception,
    Ipc,
    SchedulingEvent,
}

pub struct Message {
    pub source: ProcessIdentifier,
    pub destination: ProcessIdentifier,
    pub message_type: MessageType,
    pub payload: [u8; Self::SIZE],
}

//==================================================================================================
//  Implementations
//==================================================================================================

impl Message {
    pub const SIZE: usize = 64;

    ///
    /// # Description
    ///
    /// Creates a new message.
    ///
    /// # Parameters
    ///
    /// - `source`: The source process.
    /// - `destination`: The destination process.
    /// - `payload`: The message payload.
    ///
    /// # Returns
    ///
    /// The new message.
    ///
    pub fn new(
        source: ProcessIdentifier,
        destination: ProcessIdentifier,
        payload: [u8; Self::SIZE],
        message_type: MessageType,
    ) -> Self {
        Self {
            source,
            destination,
            payload,
            message_type,
        }
    }
}

impl Default for Message {
    fn default() -> Self {
        Self {
            source: ProcessIdentifier::KERNEL,
            destination: ProcessIdentifier::KERNEL,
            payload: [0; Self::SIZE],
            message_type: MessageType::Ipc,
        }
    }
}
