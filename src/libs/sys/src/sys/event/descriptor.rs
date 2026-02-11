// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::Error,
    event::Event,
};
use ::core::fmt::Debug;

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Default, Clone, Eq, PartialEq)]
pub struct EventDescriptor(usize);

//==================================================================================================
// Implementations
//==================================================================================================

impl EventDescriptor {
    const EVENT_MASKLEN: usize = Event::BIT_LENGTH;
    const EVENT_MASK: usize = (1 << Self::EVENT_MASKLEN) - 1;
    const EVENT_SHIFT: usize = 0;
    const ID_MASK: usize = !((1 << (usize::BITS - 1)) | Self::EVENT_MASK);
    const ID_SHIFT: usize = Self::EVENT_SHIFT + Self::EVENT_MASKLEN;

    pub fn into_raw(&self) -> usize {
        self.0
    }

    pub fn new(id: usize, ev: Event) -> Self {
        let id: usize = (id << Self::ID_SHIFT) & Self::ID_MASK;
        let ev: usize = usize::from(ev) & Self::EVENT_MASK;
        Self(id | ev)
    }

    pub fn id(&self) -> usize {
        (self.0 & Self::ID_MASK) >> Self::ID_SHIFT
    }

    pub fn event(&self) -> Event {
        Event::try_from(self.0 & Self::EVENT_MASK).unwrap()
    }

    pub fn is_interrupt(&self) -> bool {
        self.event().is_interrupt()
    }

    pub fn is_exception(&self) -> bool {
        self.event().is_exception()
    }

    pub fn to_ne_bytes(&self) -> [u8; core::mem::size_of::<usize>()] {
        self.0.to_ne_bytes()
    }

    pub fn from_ne_bytes(bytes: [u8; core::mem::size_of::<usize>()]) -> Result<Self, Error> {
        let raw: usize = usize::from_ne_bytes(bytes);
        // Validate that event bits encode a valid event and normalize descriptor.
        let ev: Event = Event::try_from(raw & Self::EVENT_MASK)?;
        let id: usize = (raw & Self::ID_MASK) >> Self::ID_SHIFT;
        Ok(Self::new(id, ev))
    }
}

impl Debug for EventDescriptor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "event_information (id={:?}, type={:?})", self.id(), self.event())
    }
}

impl From<EventDescriptor> for usize {
    fn from(eventid: EventDescriptor) -> usize {
        eventid.0
    }
}

impl TryFrom<usize> for EventDescriptor {
    type Error = Error;

    fn try_from(raw: usize) -> Result<Self, Self::Error> {
        let id: usize = (raw & EventDescriptor::ID_MASK) >> EventDescriptor::ID_SHIFT;
        let ev: Event = Event::try_from(raw & EventDescriptor::EVENT_MASK)?;
        Ok(Self::new(id, ev))
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{
        ExceptionEvent,
        InterruptEvent,
    };

    #[test]
    fn from_ne_bytes_valid_event() {
        let desc: EventDescriptor =
            EventDescriptor::new(1, Event::Interrupt(InterruptEvent::Interrupt0));
        let bytes: [u8; core::mem::size_of::<usize>()] = desc.to_ne_bytes();
        let result: Result<EventDescriptor, Error> = EventDescriptor::from_ne_bytes(bytes);
        assert!(result.is_ok());
        let restored: EventDescriptor = result.expect("should be valid");
        assert_eq!(restored.id(), 1);
        assert_eq!(restored.event(), Event::Interrupt(InterruptEvent::Interrupt0));
    }

    #[test]
    fn from_ne_bytes_invalid_event() {
        // Construct raw bytes with invalid event bits.
        let invalid_event_value: usize = EventDescriptor::EVENT_MASK;
        let bytes: [u8; core::mem::size_of::<usize>()] = invalid_event_value.to_ne_bytes();
        let result: Result<EventDescriptor, Error> = EventDescriptor::from_ne_bytes(bytes);
        assert!(result.is_err());
    }

    #[test]
    fn try_from_usize_valid() {
        let desc: EventDescriptor =
            EventDescriptor::new(2, Event::Exception(ExceptionEvent::Exception0));
        let raw: usize = desc.into_raw();
        let result: Result<EventDescriptor, Error> = EventDescriptor::try_from(raw);
        assert!(result.is_ok());
    }

    #[test]
    fn try_from_usize_invalid() {
        let invalid_raw: usize = EventDescriptor::EVENT_MASK;
        let result: Result<EventDescriptor, Error> = EventDescriptor::try_from(invalid_raw);
        assert!(result.is_err());
    }
}
