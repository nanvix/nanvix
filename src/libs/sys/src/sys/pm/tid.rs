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
/// A type that represents a thread identifier.
///
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct ThreadIdentifier(i32);
::static_assert::assert_eq_size!(ThreadIdentifier, 4);
::static_assert::assert_eq_align!(ThreadIdentifier, 4);

//==================================================================================================
// Implementations
//==================================================================================================

impl ThreadIdentifier {
    // Raw identifier for the kernel thread.
    pub const KERNEL_RAW: i32 = 0;

    /// Identifier of the kernel thread.
    pub const KERNEL: ThreadIdentifier = ThreadIdentifier(Self::KERNEL_RAW);

    /// Error message for conversion failures.
    const PARSE_ERROR_MESSAGE: &'static str = "invalid thread identifier";

    /// Identifier of the init daemon thread.
    pub const INITD: ThreadIdentifier = ThreadIdentifier(1);

    pub fn to_ne_bytes(&self) -> [u8; core::mem::size_of::<i32>()] {
        self.0.to_ne_bytes()
    }

    pub fn from_ne_bytes(bytes: [u8; core::mem::size_of::<i32>()]) -> Self {
        Self(i32::from_ne_bytes(bytes))
    }
}

impl From<ThreadIdentifier> for isize {
    fn from(tid: ThreadIdentifier) -> isize {
        tid.0 as isize
    }
}

impl From<ThreadIdentifier> for i32 {
    fn from(tid: ThreadIdentifier) -> i32 {
        tid.0
    }
}

impl From<ThreadIdentifier> for i64 {
    fn from(tid: ThreadIdentifier) -> i64 {
        tid.0 as i64
    }
}

impl TryFrom<ThreadIdentifier> for usize {
    type Error = Error;

    fn try_from(tid: ThreadIdentifier) -> Result<Self, Self::Error> {
        tid.0.try_into().map_err(|_| {
            Error::new(ErrorCode::InvalidArgument, ThreadIdentifier::PARSE_ERROR_MESSAGE)
        })
    }
}

impl TryFrom<ThreadIdentifier> for u32 {
    type Error = Error;

    fn try_from(tid: ThreadIdentifier) -> Result<Self, Self::Error> {
        tid.0.try_into().map_err(|_| {
            Error::new(ErrorCode::InvalidArgument, ThreadIdentifier::PARSE_ERROR_MESSAGE)
        })
    }
}

impl TryFrom<ThreadIdentifier> for u64 {
    type Error = Error;

    fn try_from(tid: ThreadIdentifier) -> Result<Self, Self::Error> {
        tid.0.try_into().map_err(|_| {
            Error::new(ErrorCode::InvalidArgument, ThreadIdentifier::PARSE_ERROR_MESSAGE)
        })
    }
}

impl TryFrom<isize> for ThreadIdentifier {
    type Error = Error;

    fn try_from(raw_tid: isize) -> Result<Self, Self::Error> {
        raw_tid
            .try_into()
            .map_err(|_| {
                Error::new(ErrorCode::InvalidArgument, ThreadIdentifier::PARSE_ERROR_MESSAGE)
            })
            .map(ThreadIdentifier)
    }
}

impl From<i32> for ThreadIdentifier {
    fn from(raw_tid: i32) -> ThreadIdentifier {
        ThreadIdentifier(raw_tid)
    }
}

impl TryFrom<i64> for ThreadIdentifier {
    type Error = Error;

    fn try_from(raw_tid: i64) -> Result<Self, Self::Error> {
        raw_tid
            .try_into()
            .map_err(|_| {
                Error::new(ErrorCode::InvalidArgument, ThreadIdentifier::PARSE_ERROR_MESSAGE)
            })
            .map(ThreadIdentifier)
    }
}

impl TryFrom<usize> for ThreadIdentifier {
    type Error = Error;

    fn try_from(raw_tid: usize) -> Result<Self, Self::Error> {
        raw_tid
            .try_into()
            .map_err(|_| {
                Error::new(ErrorCode::InvalidArgument, ThreadIdentifier::PARSE_ERROR_MESSAGE)
            })
            .map(ThreadIdentifier)
    }
}

impl TryFrom<u32> for ThreadIdentifier {
    type Error = Error;

    fn try_from(raw_tid: u32) -> Result<Self, Self::Error> {
        raw_tid
            .try_into()
            .map_err(|_| {
                Error::new(ErrorCode::InvalidArgument, ThreadIdentifier::PARSE_ERROR_MESSAGE)
            })
            .map(ThreadIdentifier)
    }
}

impl TryFrom<u64> for ThreadIdentifier {
    type Error = Error;

    fn try_from(raw_tid: u64) -> Result<Self, Self::Error> {
        raw_tid
            .try_into()
            .map_err(|_| {
                Error::new(ErrorCode::InvalidArgument, ThreadIdentifier::PARSE_ERROR_MESSAGE)
            })
            .map(ThreadIdentifier)
    }
}

impl core::fmt::Debug for ThreadIdentifier {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
