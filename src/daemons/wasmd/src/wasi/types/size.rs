// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::memory::{
    ReadBytes,
    ReadBytesError,
    WriteBytes,
};
use ::core::mem;

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size(u32);

//==================================================================================================
// Implementations
//==================================================================================================

impl Size {
    pub fn new(val: u32) -> Self {
        Self(val)
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

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

impl TryFrom<i32> for Size {
    type Error = ();

    fn try_from(val: i32) -> Result<Self, Self::Error> {
        if val < 0 {
            Err(())
        } else {
            Ok(Size(val as u32))
        }
    }
}

impl WriteBytes for Size {
    fn write_le_bytes(&self, to: &mut [u8]) {
        to[..mem::size_of::<Self>()].copy_from_slice(&self.0.to_le_bytes());
    }
}

impl ReadBytes for Size {
    fn read_le_bytes(from: &[u8]) -> Result<Self, ReadBytesError> {
        Ok(Self(u32::read_le_bytes(from)?))
    }
}

impl core::ops::Add for Size {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Size(self.0 + other.0)
    }
}

impl core::ops::AddAssign for Size {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}
