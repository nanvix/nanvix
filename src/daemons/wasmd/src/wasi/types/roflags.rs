// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use core::mem;

use crate::memory::WriteBytes;
//==================================================================================================
// Structures
//==================================================================================================

/// Flags returned by sock_recv.
pub struct RoFlags {
    /// Data was truncated.
    pub trunc: bool,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl RoFlags {
    const BIT_OFFSET_OF_TRUNC: u64 = 0;
}

impl From<i32> for RoFlags {
    fn from(val: i32) -> Self {
        Self {
            trunc: val & (1 << Self::BIT_OFFSET_OF_TRUNC) != 0,
        }
    }
}

impl From<i16> for RoFlags {
    fn from(val: i16) -> Self {
        Self::from(val as i32)
    }
}

impl From<&RoFlags> for u16 {
    fn from(flags: &RoFlags) -> Self {
        (flags.trunc as u16) << RoFlags::BIT_OFFSET_OF_TRUNC
    }
}

impl WriteBytes for RoFlags {
    fn write_le_bytes(&self, to: &mut [u8]) {
        let self_: u16 = self.into();
        to[..mem::size_of::<u16>()].copy_from_slice(&self_.to_le_bytes());
    }
}
