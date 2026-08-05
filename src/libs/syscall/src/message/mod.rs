// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod long;
mod part;

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::vec::Vec;
use ::sys::{
    error::Error,
    ipc::{
        Message,
        MessageType,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Exports
//==================================================================================================

pub use long::SystemCallLongMessage;
pub use part::SystemCallMessagePart;

//==================================================================================================
// Traits
//==================================================================================================

pub trait MessageSerializer
where
    Self: Sized,
{
    ///
    /// # Description
    ///
    /// Serializes the target structure into a byte array.
    ///
    /// # Returns
    ///
    /// A byte array containing the serialized structure.
    ///
    fn to_bytes(&self) -> Vec<u8>;
}

pub trait MessageDeserializer
where
    Self: Sized,
{
    ///
    /// # Description
    ///
    /// Deserializes a byte array into a structure.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array to deserialize.
    ///
    /// # Returns
    ///
    /// Upon success, the deserialized structure is returned. Upon failure, an error is returned
    /// instead.
    ///
    fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error>;
}

pub trait MessagePartitioner
where
    Self: Sized,
    Self: MessageSerializer,
    Self: MessageDeserializer,
{
    ///
    /// # Description
    ///
    /// Creates a new message part.
    ///
    /// # Parameters
    ///
    /// - `tid`: Thread identifier.
    /// - `total_parts`: Total number of parts.
    /// - `part_number`: Part number.
    /// - `payload_size`: Payload size.
    /// - `payload`: Payload.
    ///
    /// # Returns
    ///
    /// Upon success, the new message part is returned. Upon failure, an error is returned instead.
    ///
    fn new_part(
        tid: ThreadIdentifier,
        total_parts: u16,
        part_number: u16,
        payload_size: u8,
        payload: [u8; SystemCallMessagePart::PAYLOAD_SIZE],
        destination: ProcessIdentifier,
        message_type: MessageType,
    ) -> Result<Message, Error>;

    ///
    /// # Description
    ///
    /// Splits a message into parts.
    ///
    /// # Parameters
    ///
    /// - `tid`: Thread identifier.
    /// - `destination`: Process identifier of the destination daemon.
    /// - `message_type`: Message type to use.
    ///
    /// # Returns
    ///
    /// Upon success, a vector containing the message parts is returned. Upon failure, an error is
    /// returned instead.
    ///
    fn into_parts(
        self,
        tid: ThreadIdentifier,
        destination: ProcessIdentifier,
        message_type: MessageType,
    ) -> Result<Vec<Message>, Error> {
        let bytes: Vec<u8> = self.to_bytes();
        let num_parts: u16 = bytes
            .len()
            .div_ceil(SystemCallMessagePart::PAYLOAD_SIZE)
            .try_into()
            .map_err(|_| {
                Error::new(
                    ::sys::error::ErrorCode::InvalidMessage,
                    "message is too large to be partitioned",
                )
            })?;
        let mut parts: Vec<Message> = Vec::with_capacity(num_parts as usize);

        for (part_number, chunk) in bytes
            .chunks(SystemCallMessagePart::PAYLOAD_SIZE)
            .enumerate()
        {
            let mut payload = [0; SystemCallMessagePart::PAYLOAD_SIZE];
            payload[..chunk.len()].copy_from_slice(chunk);
            parts.push(Self::new_part(
                tid,
                num_parts,
                part_number as u16,
                chunk.len() as u8,
                payload,
                destination,
                message_type,
            )?);
        }

        Ok(parts)
    }

    ///
    /// # Description
    ///
    /// Processes a request.
    ///
    /// # Parameters
    ///
    /// - `source`: Source process identifier.
    /// - `request`: Request to process.
    ///
    /// # Returns
    ///
    /// Upon success, a vector containing the response messages is returned. Upon failure, an error
    /// is returned instead.
    ///
    fn from_parts(parts: &[SystemCallMessagePart]) -> Result<Self, Error> {
        let mut bytes: Vec<u8> =
            Vec::with_capacity(parts.len() * SystemCallMessagePart::PAYLOAD_SIZE);

        let expected_total: u16 = parts
            .first()
            .map(|part| part.total_parts)
            .ok_or_else(|| Error::new(::sys::error::ErrorCode::InvalidMessage, "no parts"))?;
        if expected_total as usize != parts.len() {
            return Err(Error::new(
                ::sys::error::ErrorCode::InvalidMessage,
                "incomplete multipart message",
            ));
        }

        for (expected_number, part) in parts.iter().enumerate() {
            let payload_size: usize = part.payload_size as usize;
            if part.total_parts != expected_total
                || part.part_number as usize != expected_number
                || payload_size > SystemCallMessagePart::PAYLOAD_SIZE
            {
                return Err(Error::new(
                    ::sys::error::ErrorCode::InvalidMessage,
                    "invalid multipart message",
                ));
            }
            bytes.extend_from_slice(&part.payload[..payload_size]);
        }

        Self::try_from_bytes(&bytes)
    }
}
