// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::{
        Error,
        ErrorCode,
    },
    event::EventDescriptor,
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::core::fmt::Debug;

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Default, Debug)]
pub struct EventInformation {
    pub id: EventDescriptor,
    pub pid: ProcessIdentifier,
    pub number: Option<usize>,
    pub code: Option<usize>,
    pub address: Option<usize>,
    pub instruction: Option<usize>,
}

impl From<EventInformation> for Message {
    fn from(info: EventInformation) -> Self {
        let mut payload: [u8; Message::PAYLOAD_SIZE] = [0; Message::PAYLOAD_SIZE];

        let mut offset: usize = 0;
        payload[offset..offset + core::mem::size_of::<EventDescriptor>()]
            .copy_from_slice(&info.id.to_ne_bytes());
        offset += core::mem::size_of::<EventDescriptor>();

        payload[offset..offset + core::mem::size_of::<ProcessIdentifier>()]
            .copy_from_slice(&info.pid.to_ne_bytes());
        offset += core::mem::size_of::<ProcessIdentifier>();

        // The optional fields below carry 32-bit guest quantities (exception number, error code,
        // fault address and faulting instruction). They are serialized as fixed-width `u32` rather
        // than native `usize` so the encoding is identical on the 32-bit guest (where this runs at
        // runtime) and on a 64-bit host (where it is exercised by unit tests), and so the worst
        // case fits within the message payload. On the guest `usize` is 32 bits, so this is lossless
        // and byte-for-byte identical to the previous native encoding.
        if let Some(number) = info.number {
            payload[offset..offset + core::mem::size_of::<u32>()]
                .copy_from_slice(&(number as u32).to_ne_bytes());
            offset += core::mem::size_of::<u32>();
        }

        if let Some(code) = info.code {
            payload[offset..offset + core::mem::size_of::<u32>()]
                .copy_from_slice(&(code as u32).to_ne_bytes());
            offset += core::mem::size_of::<u32>();
        }

        if let Some(address) = info.address {
            payload[offset..offset + core::mem::size_of::<u32>()]
                .copy_from_slice(&(address as u32).to_ne_bytes());
            offset += core::mem::size_of::<u32>();
        }

        if let Some(instruction) = info.instruction {
            payload[offset..offset + core::mem::size_of::<u32>()]
                .copy_from_slice(&(instruction as u32).to_ne_bytes());
        }

        Message::new(
            MessageSender::new(info.pid, ThreadIdentifier::NONE),
            MessageReceiver::new(info.pid, ThreadIdentifier::NONE),
            MessageType::Exception,
            None,
            payload,
        )
    }
}

impl EventInformation {
    /// # Description
    ///
    /// Reads exactly `N` bytes from `payload` at the current `offset`, advancing `offset`.
    ///
    /// # Parameters
    ///
    /// - `payload`: The byte slice to read from.
    /// - `offset`: The current read position, advanced by `N` on success.
    /// - `field`: A description of the field being read, used in error messages.
    ///
    /// # Returns
    ///
    /// A fixed-size byte array of length `N`.
    ///
    /// # Errors
    ///
    /// Returns `ErrorCode::InvalidMessage` if the offset overflows or the payload is too short.
    ///
    fn read_required_bytes<const N: usize>(
        payload: &[u8],
        offset: &mut usize,
        field: &'static str,
    ) -> Result<[u8; N], Error> {
        let end: usize = offset
            .checked_add(N)
            .ok_or(Error::new(ErrorCode::InvalidMessage, field))?;
        let bytes: [u8; N] = payload
            .get(*offset..end)
            .ok_or(Error::new(ErrorCode::InvalidMessage, field))?
            .try_into()
            .map_err(|_| Error::new(ErrorCode::InvalidMessage, field))?;
        *offset = end;
        Ok(bytes)
    }

    /// # Description
    ///
    /// Reads an optional fixed-width `u32` from `payload` at the current `offset`, advancing
    /// `offset`, and widens it to `usize`.
    ///
    /// # Parameters
    ///
    /// - `payload`: The byte slice to read from.
    /// - `offset`: The current read position, advanced on success.
    ///
    /// # Returns
    ///
    /// `Some(value)` if enough bytes remain, or `None` if the payload has insufficient bytes.
    ///
    fn read_optional_u32(payload: &[u8], offset: &mut usize) -> Option<usize> {
        let size: usize = core::mem::size_of::<u32>();
        let end: usize = offset.checked_add(size)?;
        if end > payload.len() {
            return None;
        }
        let bytes: [u8; core::mem::size_of::<u32>()] =
            payload.get(*offset..end)?.try_into().ok()?;
        *offset = end;
        Some(u32::from_ne_bytes(bytes) as usize)
    }
}

impl TryFrom<Message> for EventInformation {
    type Error = Error;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        let payload: &[u8] = &message.payload;
        let mut offset: usize = 0;

        let id: EventDescriptor = EventDescriptor::from_ne_bytes(Self::read_required_bytes(
            payload,
            &mut offset,
            "invalid event descriptor",
        )?)?;
        let pid: ProcessIdentifier = ProcessIdentifier::from_ne_bytes(Self::read_required_bytes(
            payload,
            &mut offset,
            "invalid process identifier",
        )?);

        let number: Option<usize> = Self::read_optional_u32(payload, &mut offset);
        let code: Option<usize> = Self::read_optional_u32(payload, &mut offset);
        let address: Option<usize> = Self::read_optional_u32(payload, &mut offset);
        let instruction: Option<usize> = Self::read_optional_u32(payload, &mut offset);

        Ok(Self {
            id,
            pid,
            number,
            code,
            address,
            instruction,
        })
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
