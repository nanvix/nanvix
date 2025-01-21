// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::memory::WriteBytes;
use ::core::mem;

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size(u32);

//==================================================================================================
// Implementations
//==================================================================================================

impl From<usize> for Size {
    fn from(val: usize) -> Self {
        Size(val as u32)
    }
}

impl From<Size> for u32 {
    fn from(size: Size) -> Self {
        size.0
    }
}

impl WriteBytes for Size {
    fn write_le_bytes(&self, to: &mut [u8]) {
        to[..mem::size_of::<Self>()].copy_from_slice(&self.0.to_le_bytes());
    }
}
