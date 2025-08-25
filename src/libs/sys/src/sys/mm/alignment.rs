// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]
#![forbid(clippy::cast_possible_truncation)]
#![forbid(clippy::cast_possible_wrap)]
#![forbid(clippy::cast_precision_loss)]
#![forbid(clippy::cast_sign_loss)]
#![forbid(clippy::char_lit_as_u8)]
#![forbid(clippy::fn_to_numeric_cast)]
#![forbid(clippy::fn_to_numeric_cast_with_truncation)]
#![forbid(clippy::ptr_as_ptr)]
#![forbid(clippy::unnecessary_cast)]
#![forbid(invalid_reference_casting)]
#![forbid(clippy::panic)]
#![forbid(clippy::unimplemented)]
#![forbid(clippy::todo)]
#![forbid(clippy::unreachable)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Alignment {
    /// Aligned to 4 bytes.
    Align4 = 4,
    /// Aligned to 8 bytes.
    Align8 = 8,
    /// Aligned to 16 bytes.
    Align16 = 16,
    /// Aligned to 32 bytes.
    Align32 = 32,
    /// Aligned to 64 bytes.
    Align64 = 64,
    /// Aligned to 128 bytes.
    Align128 = 128,
    /// Aligned to 256 bytes.
    Align256 = 256,
    /// Aligned to 512 bytes.
    Align512 = 512,
    /// Aligned to 1024 bytes.
    Align1024 = 1024,
    /// Aligned to 2048 bytes.
    Align2048 = 2048,
    /// Aligned to 4096 bytes.
    Align4096 = 4096,
    /// Aligned to 8192 bytes.
    Align8192 = 8192,
    /// Aligned to 16384 bytes.
    Align16384 = 16384,
    /// Aligned to 32768 bytes.
    Align32768 = 32768,
    /// Aligned to 65536 bytes.
    Align65536 = 65536,
    /// Aligned to 131072 bytes.
    Align131072 = 131072,
    /// Aligned to 262144 bytes.
    Align262144 = 262144,
    /// Aligned to 524288 bytes.
    Align524288 = 524288,
    /// Aligned to 1048576 bytes.
    Align1048576 = 1048576,
    /// Aligned to 2097152 bytes.
    Align2097152 = 2097152,
    /// Aligned to 4194304 bytes.
    Align4194304 = 4194304,
}

impl TryFrom<usize> for Alignment {
    type Error = Error;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            4 => Ok(Alignment::Align4),
            8 => Ok(Alignment::Align8),
            16 => Ok(Alignment::Align16),
            32 => Ok(Alignment::Align32),
            64 => Ok(Alignment::Align64),
            128 => Ok(Alignment::Align128),
            256 => Ok(Alignment::Align256),
            512 => Ok(Alignment::Align512),
            1024 => Ok(Alignment::Align1024),
            2048 => Ok(Alignment::Align2048),
            4096 => Ok(Alignment::Align4096),
            8192 => Ok(Alignment::Align8192),
            16384 => Ok(Alignment::Align16384),
            32768 => Ok(Alignment::Align32768),
            65536 => Ok(Alignment::Align65536),
            131072 => Ok(Alignment::Align131072),
            262144 => Ok(Alignment::Align262144),
            524288 => Ok(Alignment::Align524288),
            1048576 => Ok(Alignment::Align1048576),
            2097152 => Ok(Alignment::Align2097152),
            4194304 => Ok(Alignment::Align4194304),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid alignment")),
        }
    }
}

impl TryFrom<u8> for Alignment {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Alignment::try_from(value as usize)
    }
}

impl TryFrom<u16> for Alignment {
    type Error = Error;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Alignment::try_from(value as usize)
    }
}

impl TryFrom<u32> for Alignment {
    type Error = Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Alignment::try_from(value as usize)
    }
}

impl From<Alignment> for usize {
    fn from(align: Alignment) -> Self {
        align as usize
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn align_up(value: usize, align: Alignment) -> usize {
    (value + align as usize - 1) & !(align as usize - 1)
}

pub fn align_down(value: usize, align: Alignment) -> usize {
    value & !(align as usize - 1)
}

pub fn is_aligned(value: usize, align: Alignment) -> bool {
    value & (align as usize - 1) == 0
}
