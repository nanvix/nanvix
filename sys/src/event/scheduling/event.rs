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
}

//==================================================================================================
// Implementations
//==================================================================================================

impl SchedulingEvent {
    /// Number of scheduling events.
    pub const NUMBER_EVENTS: usize = 1;

    /// Scheduling events.
    pub const VALUES: [Self; Self::NUMBER_EVENTS] = [Self::ProcessTermination];
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
