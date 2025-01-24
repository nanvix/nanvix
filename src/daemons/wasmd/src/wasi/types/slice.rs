// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::wasi::types::{
    Pointer,
    Size,
};
use ::core::mem;

//==================================================================================================
// Structures
//==================================================================================================

pub struct Slice<'a, T> {
    /// Pointer to the base of the slice.
    ptr: Pointer<T>,
    /// Number of elements in the slice.
    len: Size,
    /// Underlying memory.
    memory: &'a [u8],
}

pub struct SliceError;

//==================================================================================================
// Implementations
//==================================================================================================

impl<'a, T> Slice<'a, T> {
    /// Creates from raw parts.
    pub fn for_raw_parts(memory: &'a [u8], ptr: Pointer<T>, len: Size) -> Self {
        Self { ptr, len, memory }
    }

    /// Attempts to obtain a mutable reference to the slice.
    pub fn as_mut(&mut self) -> Result<&mut [T], SliceError> {
        let base: usize = self.ptr.base().value() as usize;
        let len: usize = self.len.value() as usize;

        // Check if slice is within bounds.
        if base + len * mem::size_of::<T>() > self.memory.len() {
            return Err(SliceError);
        }

        Ok(unsafe {
            core::slice::from_raw_parts_mut(self.memory.as_ptr().add(base) as *mut T, len)
        })
    }

    /// Attempts to obtain a reference to the slice.
    pub fn as_ref(&self) -> Result<&[T], SliceError> {
        let base: usize = self.ptr.base().value() as usize;
        let len: usize = self.len.value() as usize;

        // Check if slice is within bounds.
        if base + len * mem::size_of::<T>() > self.memory.len() {
            return Err(SliceError);
        }

        Ok(unsafe { core::slice::from_raw_parts(self.memory.as_ptr().add(base) as *const T, len) })
    }
}
