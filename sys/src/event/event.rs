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
    event::{
        ExceptionEvent,
        InterruptEvent,
        SchedulingEvent,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Interrupt(InterruptEvent),
    Exception(ExceptionEvent),
    Scheduling(SchedulingEvent),
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Event {
    pub const BIT_LENGTH: usize = 7;

    pub fn is_interrupt(&self) -> bool {
        match self {
            Event::Interrupt(_) => true,
            _ => false,
        }
    }

    pub fn is_exception(&self) -> bool {
        match self {
            Event::Exception(_) => true,
            _ => false,
        }
    }
}

impl From<Event> for u32 {
    fn from(ev: Event) -> u32 {
        match ev {
            Event::Interrupt(ev) => u32::from(ev),
            Event::Exception(ev) => InterruptEvent::NUMBER_EVENTS as u32 + (u32::from(ev)),
            Event::Scheduling(ev) => {
                InterruptEvent::NUMBER_EVENTS as u32
                    + ExceptionEvent::NUMBER_EVENTS as u32
                    + u32::from(ev)
            },
        }
    }
}

impl TryFrom<u32> for Event {
    type Error = Error;

    fn try_from(raw: u32) -> Result<Self, Self::Error> {
        if raw
            > InterruptEvent::NUMBER_EVENTS as u32
                + ExceptionEvent::NUMBER_EVENTS as u32
                + SchedulingEvent::NUMBER_EVENTS as u32
        {
            return Err(Error::new(ErrorCode::InvalidArgument, "invalid event"));
        } else if raw >= InterruptEvent::NUMBER_EVENTS as u32 + ExceptionEvent::NUMBER_EVENTS as u32
        {
            Ok(Event::Scheduling(SchedulingEvent::try_from(
                raw - InterruptEvent::NUMBER_EVENTS as u32 - ExceptionEvent::NUMBER_EVENTS as u32,
            )?))
        } else if raw >= InterruptEvent::NUMBER_EVENTS as u32 {
            Ok(Event::Exception(ExceptionEvent::try_from(
                raw - InterruptEvent::NUMBER_EVENTS as u32,
            )?))
        } else {
            Ok(Event::Interrupt(InterruptEvent::try_from(raw)?))
        }
    }
}

impl From<Event> for usize {
    fn from(ev: Event) -> usize {
        u32::from(ev) as usize
    }
}

impl TryFrom<usize> for Event {
    type Error = Error;

    fn try_from(raw: usize) -> Result<Self, Self::Error> {
        Event::try_from(raw as u32)
    }
}

impl From<InterruptEvent> for Event {
    fn from(ev: InterruptEvent) -> Event {
        Event::Interrupt(ev)
    }
}

impl From<ExceptionEvent> for Event {
    fn from(ev: ExceptionEvent) -> Event {
        Event::Exception(ev)
    }
}

impl From<SchedulingEvent> for Event {
    fn from(ev: SchedulingEvent) -> Event {
        Event::Scheduling(ev)
    }
}
