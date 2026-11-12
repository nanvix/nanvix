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
/// Scheduling events, which are the process and thread lifecycle events that the kernel publishes.
///
/// Scheduling events are owned as a single class: registering any of them claims all of them, and
/// the owner receives each event as a message of the matching lifecycle
/// [`MessageType`](crate::ipc::MessageType). The kernel delivers lifecycle events and local IPC in
/// one ordered domain, in production order: the creation of a process precedes the termination of
/// each of its threads, every thread termination precedes the termination of the process, and a
/// thread termination follows every message that the thread sent before it terminated.
///
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SchedulingEvent {
    /// Termination of a process, published when its last thread terminates.
    ProcessTermination,
    /// Creation of a process, published when the creation commits.
    ProcessCreation,
    /// Termination of a thread, published when the thread first becomes a zombie.
    ThreadTermination,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl SchedulingEvent {
    /// Number of scheduling events.
    pub const NUMBER_EVENTS: usize = 3;

    /// Scheduling events, in discriminant order.
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
mod tests;
