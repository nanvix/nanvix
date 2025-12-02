// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    memory::{
        ReadBytes,
        ReadBytesError,
    },
    wasi::types::{
        Address,
        Size,
    },
};
use ::core::{
    marker::PhantomData,
    mem,
};

//==================================================================================================
// Structures
//==================================================================================================

/// An aligned pointer to a region of memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct Pointer<T> {
    /// The base address of the region.
    base: Address,
    /// Marker for the type `T``.
    _marker: PhantomData<T>,
}
::static_assert::assert_eq_align!(Pointer<u8>, 4);
::static_assert::assert_eq_size!(Pointer<u8>, 4);

pub struct UnalignedPointerError;

//==================================================================================================
// Implementations
//==================================================================================================

impl<T> Pointer<T> {
    /// Creates a new pointer to a region of memory.
    pub fn new(base: Address) -> Result<Self, UnalignedPointerError> {
        // Check if pointer is aligned.
        if !(base.value() as usize).is_multiple_of(mem::align_of::<T>()) {
            return Err(UnalignedPointerError);
        }

        Ok(Self {
            base,
            _marker: PhantomData,
        })
    }

    /// Returns the base address of the region.
    pub fn base(&self) -> Address {
        self.base
    }

    /// Returns the length of the region based on the size of `T``.
    pub fn len(&self) -> Size {
        Size::from(mem::size_of::<T>())
    }
}

impl<T> ReadBytes for Pointer<T> {
    fn read_le_bytes(from: &[u8]) -> Result<Self, ReadBytesError> {
        Ok(Self {
            base: Address::read_le_bytes(from)?,
            _marker: PhantomData,
        })
    }
}
