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
    ipc::Message,
    pm::ThreadIdentifier,
};

//==================================================================================================
// Exports
//==================================================================================================

pub use long::LinuxDaemonLongMessage;
pub use part::LinuxDaemonMessagePart;

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
        payload: [u8; LinuxDaemonMessagePart::PAYLOAD_SIZE],
    ) -> Result<Message, Error>;

    ///
    /// # Description
    ///
    /// Splits a message into parts.
    ///
    /// # Parameters
    ///
    /// - `tid`: Thread identifier.
    ///
    /// # Returns
    ///
    /// Upon success, a vector containing the message parts is returned. Upon failure, an error is
    /// returned instead.
    ///
    fn into_parts(self, tid: ThreadIdentifier) -> Result<Vec<Message>, Error> {
        let bytes: Vec<u8> = self.to_bytes();
        let num_parts: u16 = bytes
            .len()
            .div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE)
            .try_into()
            .map_err(|_| {
                Error::new(
                    ::sys::error::ErrorCode::InvalidMessage,
                    "message is too large to be partitioned",
                )
            })?;
        let mut parts: Vec<Message> = Vec::with_capacity(num_parts as usize);

        for (part_number, chunk) in bytes
            .chunks(LinuxDaemonMessagePart::PAYLOAD_SIZE)
            .enumerate()
        {
            let mut payload = [0; LinuxDaemonMessagePart::PAYLOAD_SIZE];
            payload[..chunk.len()].copy_from_slice(chunk);
            parts.push(Self::new_part(
                tid,
                num_parts,
                part_number as u16,
                chunk.len() as u8,
                payload,
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
    fn from_parts(parts: &Vec<LinuxDaemonMessagePart>) -> Result<Self, Error> {
        let mut bytes: Vec<u8> =
            Vec::with_capacity(parts.len() * LinuxDaemonMessagePart::PAYLOAD_SIZE);

        for part in parts {
            bytes.extend_from_slice(&part.payload[..part.payload_size as usize]);
        }

        Self::try_from_bytes(&bytes)
    }
}
