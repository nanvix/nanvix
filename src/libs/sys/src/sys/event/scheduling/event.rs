// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::error::{
    Error,
    ErrorCode,
};
use ::core::fmt::Debug;

//==================================================================================================
// Enumerations
//==================================================================================================

///
/// # Description
///
/// Scheduling events.
///
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SchedulingEvent {
    /// Process termination.
    ProcessTermination,
    /// Process creation.
    ProcessCreation,
    /// Thread termination.
    ThreadTermination,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl SchedulingEvent {
    /// Number of scheduling events.
    pub const NUMBER_EVENTS: usize = 3;

    /// Scheduling events.
    pub const VALUES: [Self; Self::NUMBER_EVENTS] = [
        Self::ProcessTermination,
        Self::ProcessCreation,
        Self::ThreadTermination,
    ];
}

impl From<SchedulingEvent> for u32 {
    fn from(eventid: SchedulingEvent) -> u32 {
        eventid as u32
    }
}

impl TryFrom<u32> for SchedulingEvent {
    type Error = Error;

    fn try_from(raw: u32) -> Result<Self, Self::Error> {
        match raw {
            0 => Ok(Self::ProcessTermination),
            1 => Ok(Self::ProcessCreation),
            2 => Ok(Self::ThreadTermination),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid scheduling event identifier")),
        }
    }
}

impl From<SchedulingEvent> for usize {
    fn from(eventid: SchedulingEvent) -> usize {
        eventid as usize
    }
}

impl TryFrom<usize> for SchedulingEvent {
    type Error = Error;

    fn try_from(raw: usize) -> Result<Self, Self::Error> {
        Self::try_from(raw as u32)
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ::alloc::format;

    #[test]
    fn scheduling_event_discriminants_are_stable() {
        assert_eq!(u32::from(SchedulingEvent::ProcessTermination), 0);
        assert_eq!(u32::from(SchedulingEvent::ProcessCreation), 1);
        assert_eq!(u32::from(SchedulingEvent::ThreadTermination), 2);
    }

    #[test]
    fn scheduling_events_round_trip_through_numeric_conversions() {
        for event in SchedulingEvent::VALUES {
            let raw_u32: u32 = u32::from(event);
            let raw_usize: usize = usize::from(event);

            assert_eq!(
                SchedulingEvent::try_from(raw_u32)
                    .expect("valid u32 scheduling event should parse"),
                event
            );
            assert_eq!(
                SchedulingEvent::try_from(raw_usize)
                    .expect("valid usize scheduling event should parse"),
                event
            );
        }
    }

    #[test]
    fn invalid_scheduling_event_discriminants_are_rejected() {
        assert!(SchedulingEvent::try_from(SchedulingEvent::NUMBER_EVENTS as u32).is_err());
        assert!(SchedulingEvent::try_from(SchedulingEvent::NUMBER_EVENTS).is_err());
        assert!(SchedulingEvent::try_from(u32::MAX).is_err());
        assert!(SchedulingEvent::try_from(usize::MAX).is_err());
    }

    #[test]
    fn thread_termination_scheduling_event_is_formatted() {
        assert_eq!(format!("{:?}", SchedulingEvent::ThreadTermination), "ThreadTermination");
    }
}
