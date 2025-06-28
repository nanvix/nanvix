// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::char_lit_as_u8,
    clippy::fn_to_numeric_cast,
    clippy::fn_to_numeric_cast_with_truncation,
    clippy::ptr_as_ptr,
    clippy::unnecessary_cast,
    invalid_reference_casting
)]

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

///
/// # Description
///
/// A type that represents a process identifier.
///
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct ProcessIdentifier(i32);
::static_assert::assert_eq_size!(ProcessIdentifier, 4);
::static_assert::assert_eq_align!(ProcessIdentifier, 4);

//==================================================================================================
// Implementations
//==================================================================================================

impl ProcessIdentifier {
    // Raw identifier for the kernel process.
    pub const KERNEL_RAW: i32 = 0;

    /// Identifier of the kernel process.
    pub const KERNEL: ProcessIdentifier = ProcessIdentifier(Self::KERNEL_RAW);

    /// Error message for conversion failures.
    const PARSE_ERROR_MESSAGE: &'static str = "invalid process identifier";

    /// Identifier of the init daemon process.
    pub const INITD: ProcessIdentifier = ProcessIdentifier(1);

    pub fn to_ne_bytes(&self) -> [u8; core::mem::size_of::<i32>()] {
        self.0.to_ne_bytes()
    }

    pub fn from_ne_bytes(bytes: [u8; core::mem::size_of::<i32>()]) -> Self {
        Self(i32::from_ne_bytes(bytes))
    }
}

impl From<ProcessIdentifier> for isize {
    fn from(tid: ProcessIdentifier) -> isize {
        tid.0 as isize
    }
}

impl From<ProcessIdentifier> for i32 {
    fn from(tid: ProcessIdentifier) -> i32 {
        tid.0
    }
}

impl From<ProcessIdentifier> for i64 {
    fn from(tid: ProcessIdentifier) -> i64 {
        tid.0 as i64
    }
}

impl TryFrom<ProcessIdentifier> for usize {
    type Error = Error;

    fn try_from(tid: ProcessIdentifier) -> Result<Self, Self::Error> {
        tid.0.try_into().map_err(|_| {
            Error::new(ErrorCode::InvalidArgument, ProcessIdentifier::PARSE_ERROR_MESSAGE)
        })
    }
}

impl TryFrom<ProcessIdentifier> for u32 {
    type Error = Error;

    fn try_from(tid: ProcessIdentifier) -> Result<Self, Self::Error> {
        tid.0.try_into().map_err(|_| {
            Error::new(ErrorCode::InvalidArgument, ProcessIdentifier::PARSE_ERROR_MESSAGE)
        })
    }
}

impl TryFrom<ProcessIdentifier> for u64 {
    type Error = Error;

    fn try_from(tid: ProcessIdentifier) -> Result<Self, Self::Error> {
        tid.0.try_into().map_err(|_| {
            Error::new(ErrorCode::InvalidArgument, ProcessIdentifier::PARSE_ERROR_MESSAGE)
        })
    }
}

impl TryFrom<isize> for ProcessIdentifier {
    type Error = Error;

    fn try_from(raw_tid: isize) -> Result<Self, Self::Error> {
        raw_tid
            .try_into()
            .map_err(|_| {
                Error::new(ErrorCode::InvalidArgument, ProcessIdentifier::PARSE_ERROR_MESSAGE)
            })
            .map(ProcessIdentifier)
    }
}

impl From<i32> for ProcessIdentifier {
    fn from(raw_tid: i32) -> ProcessIdentifier {
        ProcessIdentifier(raw_tid)
    }
}

impl TryFrom<i64> for ProcessIdentifier {
    type Error = Error;

    fn try_from(raw_tid: i64) -> Result<Self, Self::Error> {
        raw_tid
            .try_into()
            .map_err(|_| {
                Error::new(ErrorCode::InvalidArgument, ProcessIdentifier::PARSE_ERROR_MESSAGE)
            })
            .map(ProcessIdentifier)
    }
}

impl TryFrom<usize> for ProcessIdentifier {
    type Error = Error;

    fn try_from(raw_tid: usize) -> Result<Self, Self::Error> {
        raw_tid
            .try_into()
            .map_err(|_| {
                Error::new(ErrorCode::InvalidArgument, ProcessIdentifier::PARSE_ERROR_MESSAGE)
            })
            .map(ProcessIdentifier)
    }
}

impl TryFrom<u32> for ProcessIdentifier {
    type Error = Error;

    fn try_from(raw_tid: u32) -> Result<Self, Self::Error> {
        raw_tid
            .try_into()
            .map_err(|_| {
                Error::new(ErrorCode::InvalidArgument, ProcessIdentifier::PARSE_ERROR_MESSAGE)
            })
            .map(ProcessIdentifier)
    }
}

impl TryFrom<u64> for ProcessIdentifier {
    type Error = Error;

    fn try_from(raw_tid: u64) -> Result<Self, Self::Error> {
        raw_tid
            .try_into()
            .map_err(|_| {
                Error::new(ErrorCode::InvalidArgument, ProcessIdentifier::PARSE_ERROR_MESSAGE)
            })
            .map(ProcessIdentifier)
    }
}

impl core::fmt::Debug for ProcessIdentifier {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
