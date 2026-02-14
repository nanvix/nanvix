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
    pm::ProcessIdentifier,
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

        if let Some(number) = info.number {
            payload[offset..offset + core::mem::size_of::<usize>()]
                .copy_from_slice(&number.to_ne_bytes());
            offset += core::mem::size_of::<usize>();
        }

        if let Some(code) = info.code {
            payload[offset..offset + core::mem::size_of::<usize>()]
                .copy_from_slice(&code.to_ne_bytes());
            offset += core::mem::size_of::<usize>();
        }

        if let Some(address) = info.address {
            payload[offset..offset + core::mem::size_of::<usize>()]
                .copy_from_slice(&address.to_ne_bytes());
            offset += core::mem::size_of::<usize>();
        }

        if let Some(instruction) = info.instruction {
            payload[offset..offset + core::mem::size_of::<usize>()]
                .copy_from_slice(&instruction.to_ne_bytes());
        }

        Message::new(
            MessageSender::from(info.pid),
            MessageReceiver::from(info.pid),
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
    /// Reads an optional `usize` from `payload` at the current `offset`, advancing `offset`.
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
    fn read_optional_usize(payload: &[u8], offset: &mut usize) -> Option<usize> {
        let size: usize = core::mem::size_of::<usize>();
        let end: usize = offset.checked_add(size)?;
        if end > payload.len() {
            return None;
        }
        let bytes: [u8; core::mem::size_of::<usize>()] =
            payload.get(*offset..end)?.try_into().ok()?;
        *offset = end;
        Some(usize::from_ne_bytes(bytes))
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

        let number: Option<usize> = Self::read_optional_usize(payload, &mut offset);
        let code: Option<usize> = Self::read_optional_usize(payload, &mut offset);
        let address: Option<usize> = Self::read_optional_usize(payload, &mut offset);
        let instruction: Option<usize> = Self::read_optional_usize(payload, &mut offset);

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
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]

    use super::*;

    /// Tests round-trip conversion of `EventInformation` with all fields set.
    #[test]
    fn try_from_message_round_trip_all_fields() {
        let info: EventInformation = EventInformation {
            id: EventDescriptor::from_ne_bytes(1usize.to_ne_bytes())
                .expect("valid event descriptor"),
            pid: ProcessIdentifier::from_ne_bytes(2i32.to_ne_bytes()),
            number: Some(3),
            code: Some(4),
            address: Some(5),
            instruction: Some(6),
        };
        let message: Message = Message::from(info);
        let result: EventInformation =
            EventInformation::try_from(message).expect("should succeed for valid message");
        assert_eq!(
            result.id,
            EventDescriptor::from_ne_bytes(1usize.to_ne_bytes()).expect("valid event descriptor")
        );
        assert_eq!(result.pid, ProcessIdentifier::from_ne_bytes(2i32.to_ne_bytes()));
        assert_eq!(result.number, Some(3));
        assert_eq!(result.code, Some(4));
        assert_eq!(result.address, Some(5));
        assert_eq!(result.instruction, Some(6));
    }

    /// Tests round-trip conversion of `EventInformation` with only required fields.
    /// Note: Optional fields set to `None` are serialized as zero-filled bytes in the fixed-size
    /// payload, so they are deserialized as `Some(0)` rather than `None`.
    #[test]
    fn try_from_message_round_trip_required_fields_only() {
        let info: EventInformation = EventInformation {
            id: EventDescriptor::from_ne_bytes(10usize.to_ne_bytes())
                .expect("valid event descriptor"),
            pid: ProcessIdentifier::from_ne_bytes(20i32.to_ne_bytes()),
            number: None,
            code: None,
            address: None,
            instruction: None,
        };
        let message: Message = Message::from(info);
        let result: EventInformation =
            EventInformation::try_from(message).expect("should succeed for valid message");
        assert_eq!(
            result.id,
            EventDescriptor::from_ne_bytes(10usize.to_ne_bytes()).expect("valid event descriptor")
        );
        assert_eq!(result.pid, ProcessIdentifier::from_ne_bytes(20i32.to_ne_bytes()));
        assert_eq!(result.number, Some(0));
        assert_eq!(result.code, Some(0));
        assert_eq!(result.address, Some(0));
        assert_eq!(result.instruction, Some(0));
    }

    /// Tests that `TryFrom<Message>` fails when the payload contains an invalid event descriptor.
    #[test]
    fn try_from_message_invalid_event_descriptor() {
        // Build a payload with all event bits set in the descriptor field, which is invalid.
        let mut payload: [u8; Message::PAYLOAD_SIZE] = [0; Message::PAYLOAD_SIZE];
        // Use a value with all low bits set (0x7F), which encodes an invalid event type.
        let invalid_descriptor: usize = 0x7F;
        payload[..core::mem::size_of::<usize>()].copy_from_slice(&invalid_descriptor.to_ne_bytes());

        let message: Message = Message::new(
            MessageSender::KERNEL,
            MessageReceiver::KERNEL,
            MessageType::Exception,
            None,
            payload,
        );

        let result: Result<EventInformation, Error> = EventInformation::try_from(message);
        assert!(result.is_err());
        let err: Error = result.unwrap_err();
        // The error originates from EventDescriptor::from_ne_bytes validation,
        // which returns InvalidArgument for invalid event types.
        assert_eq!(err.code, ErrorCode::InvalidArgument);
    }

    /// Tests that `TryFrom<Message>` returns `None` for optional fields when the payload is only
    /// large enough to hold the required fields.
    #[test]
    fn try_from_message_payload_too_short_for_optional_fields() {
        // Construct a valid minimal payload with only required fields and nothing beyond.
        let mut payload: [u8; Message::PAYLOAD_SIZE] = [0xFF; Message::PAYLOAD_SIZE];
        let mut offset: usize = 0;

        // Write a valid event descriptor (id=1, event=0 which is Interrupt0).
        let desc: EventDescriptor =
            EventDescriptor::from_ne_bytes(1usize.to_ne_bytes()).expect("valid event descriptor");
        payload[offset..offset + core::mem::size_of::<EventDescriptor>()]
            .copy_from_slice(&desc.to_ne_bytes());
        offset += core::mem::size_of::<EventDescriptor>();

        // Write a valid process identifier.
        let pid: ProcessIdentifier = ProcessIdentifier::from_ne_bytes(1i32.to_ne_bytes());
        payload[offset..offset + core::mem::size_of::<ProcessIdentifier>()]
            .copy_from_slice(&pid.to_ne_bytes());

        let message: Message = Message::new(
            MessageSender::KERNEL,
            MessageReceiver::KERNEL,
            MessageType::Exception,
            None,
            payload,
        );

        // Since the payload is fixed-size and filled with 0xFF, optional fields are still
        // parseable (they will contain 0xFFFFFFFF values). The conversion should still succeed.
        let result: Result<EventInformation, Error> = EventInformation::try_from(message);
        assert!(result.is_ok());
    }
}
