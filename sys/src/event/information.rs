// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    event::EventDescriptor,
    ipc::{
        Message,
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
        let mut payload: [u8; Message::SIZE] = [0; Message::SIZE];

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

        Message::new(info.pid, info.pid, payload, MessageType::Exception)
    }
}

impl From<Message> for EventInformation {
    fn from(message: Message) -> Self {
        let mut offset: usize = 0;
        let id: EventDescriptor = EventDescriptor::from_ne_bytes(
            message.payload[offset..offset + core::mem::size_of::<EventDescriptor>()]
                .try_into()
                .unwrap(),
        );
        offset += core::mem::size_of::<EventDescriptor>();

        let pid: ProcessIdentifier = ProcessIdentifier::from_ne_bytes(
            message.payload[offset..offset + core::mem::size_of::<ProcessIdentifier>()]
                .try_into()
                .unwrap(),
        );
        offset += core::mem::size_of::<ProcessIdentifier>();

        let number: Option<usize> = if offset < message.payload.len() {
            let number: usize = usize::from_ne_bytes(
                message.payload[offset..offset + core::mem::size_of::<usize>()]
                    .try_into()
                    .unwrap(),
            );
            offset += core::mem::size_of::<usize>();
            Some(number)
        } else {
            None
        };

        let code: Option<usize> = if offset < message.payload.len() {
            let code: usize = usize::from_ne_bytes(
                message.payload[offset..offset + core::mem::size_of::<usize>()]
                    .try_into()
                    .unwrap(),
            );
            offset += core::mem::size_of::<usize>();
            Some(code)
        } else {
            None
        };

        let address: Option<usize> = if offset < message.payload.len() {
            let address: usize = usize::from_ne_bytes(
                message.payload[offset..offset + core::mem::size_of::<usize>()]
                    .try_into()
                    .unwrap(),
            );
            offset += core::mem::size_of::<usize>();
            Some(address)
        } else {
            None
        };

        let instruction: Option<usize> = if offset < message.payload.len() {
            let instruction: usize = usize::from_ne_bytes(
                message.payload[offset..offset + core::mem::size_of::<usize>()]
                    .try_into()
                    .unwrap(),
            );
            Some(instruction)
        } else {
            None
        };

        Self {
            id,
            pid,
            number,
            code,
            address,
            instruction,
        }
    }
}
