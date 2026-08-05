//==================================================================================================
// Imports
//==================================================================================================

use crate::message::SystemCallMessagePart;
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
pub struct SystemCallLongMessage {
    /// Maximum number of parts that the message can contain.
    capacity: usize,
    /// Parts of the message.
    parts: Vec<SystemCallMessagePart>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl SystemCallLongMessage {
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
    pub fn add_part(&mut self, part: SystemCallMessagePart) -> Result<(), Error> {
        let total_parts: u16 = part.total_parts;
        let part_number: u16 = part.part_number;
        let payload_size: usize = part.payload_size as usize;

        if total_parts == 0
            || total_parts as usize > self.capacity
            || part_number >= total_parts
            || payload_size > SystemCallMessagePart::PAYLOAD_SIZE
        {
            return Err(Error::new(ErrorCode::InvalidMessage, "invalid message part"));
        }

        if self.parts.iter().any(|existing| {
            existing.total_parts != total_parts || existing.part_number == part_number
        }) {
            return Err(Error::new(
                ErrorCode::InvalidMessage,
                "inconsistent or duplicate message part",
            ));
        }

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
    pub fn take_parts(self) -> Vec<SystemCallMessagePart> {
        self.parts
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn part(total_parts: u16, part_number: u16, payload_size: u8) -> SystemCallMessagePart {
        SystemCallMessagePart {
            total_parts,
            part_number,
            payload_size,
            payload: [0; SystemCallMessagePart::PAYLOAD_SIZE],
        }
    }

    #[test]
    fn rejects_oversized_part_payload() {
        let mut message: SystemCallLongMessage =
            SystemCallLongMessage::new(1).expect("capacity should be valid");
        let oversized: u8 = (SystemCallMessagePart::PAYLOAD_SIZE + 1) as u8;

        let error: Error = message
            .add_part(part(1, 0, oversized))
            .expect_err("oversized payload should be rejected");
        assert_eq!(error.code, ErrorCode::InvalidMessage);
    }

    #[test]
    fn rejects_duplicate_part_number() {
        let mut message: SystemCallLongMessage =
            SystemCallLongMessage::new(2).expect("capacity should be valid");
        message
            .add_part(part(2, 0, 1))
            .expect("first part should be accepted");

        let error: Error = message
            .add_part(part(2, 0, 1))
            .expect_err("duplicate part should be rejected");
        assert_eq!(error.code, ErrorCode::InvalidMessage);
    }
}
