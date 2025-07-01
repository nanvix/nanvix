//==================================================================================================
// Imports
//==================================================================================================

use crate::message::LinuxDaemonMessagePart;
use ::alloc::vec::Vec;
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// This structure represents a long message that is split into multiple parts.
///
pub struct LinuxDaemonLongMessage {
    /// Maximum number of parts that the message can contain.
    capacity: usize,
    /// Parts of the message.
    parts: Vec<LinuxDaemonMessagePart>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl LinuxDaemonLongMessage {
    ///
    /// # Description
    ///
    /// Creates a new long message.
    ///
    /// # Parameters
    ///
    /// - `capacity`: Maximum number of parts that the message can contain.
    ///
    /// # Returns
    ///
    /// Upon success, the function returns the new long message. Otherwise, it returns an error.
    ///
    pub fn new(capacity: usize) -> Result<Self, Error> {
        // Check if capacity is invalid.
        if capacity == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument, "invalid capacity"));
        }

        Ok(Self {
            capacity,
            parts: Vec::with_capacity(capacity),
        })
    }

    ///
    /// # Description
    ///
    /// Adds a part to the long message.
    ///
    /// # Parameters
    ///
    /// - `part`: Part to add.
    ///
    /// # Returns
    ///
    /// Upon success, the function returns empty. Otherwise, it returns an error.
    ///
    pub fn add_part(&mut self, part: LinuxDaemonMessagePart) -> Result<(), Error> {
        // Check if we reached the maximum capacity.
        if self.parts.len() == self.capacity {
            return Err(Error::new(ErrorCode::MessageTooLong, "message too long"));
        }

        // Check if message is already complete.
        if self.is_complete() {
            return Err(Error::new(ErrorCode::InvalidMessage, "message is already complete"));
        }

        self.parts.push(part);

        // Keep parts sorted by part number. As vector is almost sorted, this has a linear performance.
        // TODO: reduce number of copies by manually keeping this property using a linked list.
        self.parts.sort_by_key(|part| part.part_number);

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Indicates if the message is complete.
    ///
    /// # Returns
    ///
    /// Returns `true` if the message is complete. Otherwise, it returns `false`.
    ///
    pub fn is_complete(&self) -> bool {
        if let Some(last) = self.parts.last() {
            // Check if the last part is the last part of the message.
            if last.total_parts == self.parts.len() as u16 {
                return true;
            }
        }
        false
    }

    ///
    /// # Description
    ///
    /// Takes the parts of the message.
    ///
    /// # Returns
    ///
    /// Returns the parts of the message.
    ///
    pub fn take_parts(self) -> Vec<LinuxDaemonMessagePart> {
        self.parts
    }
}
