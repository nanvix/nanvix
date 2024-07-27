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
pub enum ExceptionEvent {
    Exception0,
    Exception1,
    Exception2,
    Exception3,
    Exception4,
    Exception5,
    Exception6,
    Exception7,
    Exception8,
    Exception9,
    Exception10,
    Exception11,
    Exception12,
    Exception13,
    Exception14,
    Exception15,
    Exception16,
    Exception17,
    Exception18,
    Exception19,
    Exception20,
    Exception21,
    Exception22,
    Exception23,
    Exception24,
    Exception25,
    Exception26,
    Exception27,
    Exception28,
    Exception29,
    Exception30,
    Exception31,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl ExceptionEvent {
    pub const VALUES: [Self; 32] = [
        Self::Exception0,
        Self::Exception1,
        Self::Exception2,
        Self::Exception3,
        Self::Exception4,
        Self::Exception5,
        Self::Exception6,
        Self::Exception7,
        Self::Exception8,
        Self::Exception9,
        Self::Exception10,
        Self::Exception11,
        Self::Exception12,
        Self::Exception13,
        Self::Exception14,
        Self::Exception15,
        Self::Exception16,
        Self::Exception17,
        Self::Exception18,
        Self::Exception19,
        Self::Exception20,
        Self::Exception21,
        Self::Exception22,
        Self::Exception23,
        Self::Exception24,
        Self::Exception25,
        Self::Exception26,
        Self::Exception27,
        Self::Exception28,
        Self::Exception29,
        Self::Exception30,
        Self::Exception31,
    ];
}

impl From<ExceptionEvent> for u32 {
    fn from(eventid: ExceptionEvent) -> u32 {
        eventid as u32
    }
}

impl TryFrom<u32> for ExceptionEvent {
    type Error = Error;

    fn try_from(raw: u32) -> Result<Self, Self::Error> {
        match raw {
            0 => Ok(Self::Exception0),
            1 => Ok(Self::Exception1),
            2 => Ok(Self::Exception2),
            3 => Ok(Self::Exception3),
            4 => Ok(Self::Exception4),
            5 => Ok(Self::Exception5),
            6 => Ok(Self::Exception6),
            7 => Ok(Self::Exception7),
            8 => Ok(Self::Exception8),
            9 => Ok(Self::Exception9),
            10 => Ok(Self::Exception10),
            11 => Ok(Self::Exception11),
            12 => Ok(Self::Exception12),
            13 => Ok(Self::Exception13),
            14 => Ok(Self::Exception14),
            15 => Ok(Self::Exception15),
            16 => Ok(Self::Exception16),
            17 => Ok(Self::Exception17),
            18 => Ok(Self::Exception18),
            19 => Ok(Self::Exception19),
            20 => Ok(Self::Exception20),
            21 => Ok(Self::Exception21),
            22 => Ok(Self::Exception22),
            23 => Ok(Self::Exception23),
            24 => Ok(Self::Exception24),
            25 => Ok(Self::Exception25),
            26 => Ok(Self::Exception26),
            27 => Ok(Self::Exception27),
            28 => Ok(Self::Exception28),
            29 => Ok(Self::Exception29),
            30 => Ok(Self::Exception30),
            31 => Ok(Self::Exception31),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid exception event identifier")),
        }
    }
}

impl From<ExceptionEvent> for usize {
    fn from(eventid: ExceptionEvent) -> usize {
        eventid as usize
    }
}

impl TryFrom<usize> for ExceptionEvent {
    type Error = Error;

    fn try_from(raw: usize) -> Result<Self, Self::Error> {
        Self::try_from(raw as u32)
    }
}
