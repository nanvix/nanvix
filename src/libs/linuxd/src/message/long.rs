//==================================================================================================
// Imports
//==================================================================================================

use crate::message::LinuxDaemonMessagePart;
use ::alloc::vec::Vec;
use ::nvx::sys::error::{
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
    /// Indicates if the message contains all its parts.
    is_complete: bool,
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
            is_complete: false,
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
        if self.is_complete {
            return Err(Error::new(ErrorCode::InvalidMessage, "message is already complete"));
        }

        let part_number: u32 = part.part_number;
        self.check_out_of_order(part_number)?;
        self.parts.push(part);

        // Check if received all parts.
        if part_number == 0 {
            self.is_complete = true;
        }

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
        self.is_complete
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

    ///
    /// # Description
    ///
    /// Checks if the part number is out of order.
    ///
    /// # Parameters
    ///
    /// - `part_number`: Part number.
    ///
    /// # Returns
    ///
    /// Upon success, the function returns empty. Otherwise, it returns an error.
    ///
    fn check_out_of_order(&self, part_number: u32) -> Result<(), Error> {
        if let Some(last) = self.parts.last() {
            if last.part_number != part_number + 1 {
                return Err(Error::new(ErrorCode::InvalidMessage, "out of order segment"));
            }
        }

        Ok(())
    }
}
