// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//===================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};
use sysapi::sys_types::off_t;

//==================================================================================================
// File Offset
//==================================================================================================

///
/// # Description
///
/// A structure that represents the offset within a file.
///
#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
pub struct RegularFileOffset(off_t);

impl RegularFileOffset {
    ///
    /// # Description
    ///
    /// Performs a checked addition of two `RegularFileOffset` values.
    ///
    /// # Parameters
    ///
    /// - `other`: The other `RegularFileOffset` to add.
    ///
    /// # Returns
    ///
    /// Returns `Some(RegularFileOffset)` if the addition does not overflow, or `None` if it does.
    ///
    pub fn checked_add(self, other: RegularFileOffset) -> Option<RegularFileOffset> {
        let result = self.0.checked_add(other.0)?;
        Some(RegularFileOffset(result))
    }
}

impl From<i32> for RegularFileOffset {
    fn from(offset: i32) -> RegularFileOffset {
        static_assert::assert_eq!(core::mem::size_of::<i32>() <= core::mem::size_of::<off_t>());
        // The following conversion is safe because `i32` can be safely cast to `off_t`.
        RegularFileOffset(offset as off_t)
    }
}

impl From<isize> for RegularFileOffset {
    fn from(offset: isize) -> RegularFileOffset {
        static_assert::assert_eq!(core::mem::size_of::<isize>() <= core::mem::size_of::<off_t>());
        // The following conversion is safe because `isize` can be safely cast to `off_t`.
        RegularFileOffset(offset as off_t)
    }
}

impl From<off_t> for RegularFileOffset {
    fn from(offset: off_t) -> RegularFileOffset {
        RegularFileOffset(offset)
    }
}

impl From<RegularFileOffset> for off_t {
    fn from(offset: RegularFileOffset) -> off_t {
        offset.0
    }
}

impl TryFrom<RegularFileOffset> for usize {
    type Error = Error;
    fn try_from(offset: RegularFileOffset) -> Result<usize, Error> {
        offset
            .0
            .try_into()
            .map_err(|_| Error::new(ErrorCode::InvalidArgument, "failed to convert offset"))
    }
}
