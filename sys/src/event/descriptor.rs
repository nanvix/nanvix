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
    const ID_MASK: usize = !(1 << (usize::BITS - 1) | Self::EVENT_MASK);
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
