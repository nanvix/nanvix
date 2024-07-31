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
// Structures
//==================================================================================================

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InterruptEvent {
    Interrupt0,
    Interrupt1,
    Interrupt2,
    Interrupt3,
    Interrupt4,
    Interrupt5,
    Interrupt6,
    Interrupt7,
    Interrupt8,
    Interrupt9,
    Interrupt10,
    Interrupt11,
    Interrupt12,
    Interrupt13,
    Interrupt14,
    Interrupt15,
    Interrupt16,
    Interrupt17,
    Interrupt18,
    Interrupt19,
    Interrupt20,
    Interrupt21,
    Interrupt22,
    Interrupt23,
    Interrupt24,
    Interrupt25,
    Interrupt26,
    Interrupt27,
    Interrupt28,
    Interrupt29,
    Interrupt30,
    Interrupt31,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl InterruptEvent {
    pub const NUMBER_EVENTS: usize = 32;

    pub const VALUES: [Self; Self::NUMBER_EVENTS] = [
        Self::Interrupt0,
        Self::Interrupt1,
        Self::Interrupt2,
        Self::Interrupt3,
        Self::Interrupt4,
        Self::Interrupt5,
        Self::Interrupt6,
        Self::Interrupt7,
        Self::Interrupt8,
        Self::Interrupt9,
        Self::Interrupt10,
        Self::Interrupt11,
        Self::Interrupt12,
        Self::Interrupt13,
        Self::Interrupt14,
        Self::Interrupt15,
        Self::Interrupt16,
        Self::Interrupt17,
        Self::Interrupt18,
        Self::Interrupt19,
        Self::Interrupt20,
        Self::Interrupt21,
        Self::Interrupt22,
        Self::Interrupt23,
        Self::Interrupt24,
        Self::Interrupt25,
        Self::Interrupt26,
        Self::Interrupt27,
        Self::Interrupt28,
        Self::Interrupt29,
        Self::Interrupt30,
        Self::Interrupt31,
    ];
}

impl From<InterruptEvent> for u32 {
    fn from(eventid: InterruptEvent) -> u32 {
        eventid as u32
    }
}

impl TryFrom<u32> for InterruptEvent {
    type Error = Error;

    fn try_from(raw: u32) -> Result<Self, Self::Error> {
        match raw {
            0 => Ok(Self::Interrupt0),
            1 => Ok(Self::Interrupt1),
            2 => Ok(Self::Interrupt2),
            3 => Ok(Self::Interrupt3),
            4 => Ok(Self::Interrupt4),
            5 => Ok(Self::Interrupt5),
            6 => Ok(Self::Interrupt6),
            7 => Ok(Self::Interrupt7),
            8 => Ok(Self::Interrupt8),
            9 => Ok(Self::Interrupt9),
            10 => Ok(Self::Interrupt10),
            11 => Ok(Self::Interrupt11),
            12 => Ok(Self::Interrupt12),
            13 => Ok(Self::Interrupt13),
            14 => Ok(Self::Interrupt14),
            15 => Ok(Self::Interrupt15),
            16 => Ok(Self::Interrupt16),
            17 => Ok(Self::Interrupt17),
            18 => Ok(Self::Interrupt18),
            19 => Ok(Self::Interrupt19),
            20 => Ok(Self::Interrupt20),
            21 => Ok(Self::Interrupt21),
            22 => Ok(Self::Interrupt22),
            23 => Ok(Self::Interrupt23),
            24 => Ok(Self::Interrupt24),
            25 => Ok(Self::Interrupt25),
            26 => Ok(Self::Interrupt26),
            27 => Ok(Self::Interrupt27),
            28 => Ok(Self::Interrupt28),
            29 => Ok(Self::Interrupt29),
            30 => Ok(Self::Interrupt30),
            31 => Ok(Self::Interrupt31),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid interrupt event identifier")),
        }
    }
}

impl From<InterruptEvent> for usize {
    fn from(eventid: InterruptEvent) -> usize {
        eventid as usize
    }
}

impl TryFrom<usize> for InterruptEvent {
    type Error = Error;

    fn try_from(raw: usize) -> Result<Self, Self::Error> {
        Self::try_from(raw as u32)
    }
}
