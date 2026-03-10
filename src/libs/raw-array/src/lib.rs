// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;

//==================================================================================================
// Imports
//==================================================================================================

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
        use std::alloc;
        use alloc::{
            alloc,
            dealloc,
        };
    } else {
        extern crate alloc;
        use alloc::alloc::{
            alloc,
            dealloc,
        };
    }
}

use ::core::{
    alloc::Layout,
    ops::{
        Deref,
        DerefMut,
    },
    ptr,
    slice,
};
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::vstd::prelude::*;

// Include specifications.
#[cfg(verus_keep_ghost)]
include!("lib.spec.rs");

//==================================================================================================
// Raw Array Storage
//==================================================================================================

///
/// # Description
///
/// A type that represents the backing storage of a [`RawArray`].
///
#[derive(Debug)]
enum RawArrayStorage<T> {
    /// A storage area that is managed by [alloc::GlobalAlloc].
    Managed { ptr: ptr::NonNull<T>, len: usize },
    /// A storage area that is not managed by [alloc::GlobalAlloc].
    Unmanaged { ptr: ptr::NonNull<T>, len: usize },
}

impl<T> RawArrayStorage<T> {
    ///
    /// # Description
    ///
    /// Constructs backing storage for a raw array.
    ///
    /// # Parameters
    ///
    /// - `len`: Length of the backing storage.
    ///
    /// # Returns
    ///
    /// On success, the backing storage is returned, with all bits set to zero.
    /// On failure, an error is returned instead.
    ///
    fn new_managed(len: usize) -> Result<RawArrayStorage<T>, Error> {
        // Check if the length is invalid.
        if len == 0 || len >= i32::MAX as usize {
            return Err(Error::new(ErrorCode::InvalidArgument, "invalid length"));
        }

        // Allocate underlying memory.
        let layout: Layout = match Layout::array::<T>(len) {
            Ok(layout) => layout,
            Err(_) => return Err(Error::new(ErrorCode::InvalidArgument, "invalid layout")),
        };
        let ptr: ptr::NonNull<T> = {
            let ptr: *mut u8 = unsafe { alloc(layout) };
            match ptr::NonNull::new(ptr as *mut T) {
                Some(p) => p,
                None => {
                    return Err(Error::new(ErrorCode::OutOfMemory, "out of memory"));
                },
            }
        };

        // Initialize the backing storage.
        // Safety: The memory region is valid and the length is valid.
        unsafe { ptr::write_bytes(ptr.as_ptr(), 0, len) };

        Ok(RawArrayStorage::Managed { ptr, len })
    }

    ///
    /// # Description
    ///
    /// Constructs an unmanaged backing storage for a raw array.
    ///
    /// # Parameters
    ///
    /// - `ptr`: Pointer to the backing storage.
    /// - `len`: Length of the backing storage.
    ///
    /// # Returns
    ///
    /// On success, the backing storage is returned, with all bits set to zero.
    /// On failure, an error is returned instead.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if any of the following conditions are violated:
    ///
    /// - `ptr` must be valid for both reads and writes for `len * mem::size_of::<T>()` many bytes.
    /// - `ptr` must be properly aligned.
    /// - `ptr` must point to len consecutive properly initialized values of type `T``.
    ///
    unsafe fn new_unmanaged(ptr: *mut T, len: usize) -> Result<RawArrayStorage<T>, Error> {
        // Check if the length is invalid.
        if len == 0 || len >= i32::MAX as usize {
            return Err(Error::new(ErrorCode::InvalidArgument, "invalid length"));
        }

        // Check if memory region wraps around.
        if ptr.wrapping_add(len) < ptr {
            return Err(Error::new(ErrorCode::InvalidArgument, "wrapping memory region"));
        }

        // Check and cast provided slice.
        let ptr: ptr::NonNull<T> = match ptr::NonNull::new(ptr) {
            Some(ptr) => ptr,
            None => return Err(Error::new(ErrorCode::InvalidArgument, "invalid pointer")),
        };

        // Initialize the backing storage.
        ptr::write_bytes(ptr.as_ptr(), 0, len);

        Ok(RawArrayStorage::Unmanaged { ptr, len })
    }

    ///
    /// # Description
    ///
    /// Gets a mutable slice to the underlying data in the backing storage.
    ///
    /// # Returns
    ///
    /// A mutable slice to the underlying data in the backing storage.
    ///
    fn get_mut(&mut self) -> &mut [T] {
        match self {
            RawArrayStorage::Managed { ptr, len } => unsafe {
                slice::from_raw_parts_mut(ptr.as_ptr(), *len)
            },
            RawArrayStorage::Unmanaged { ptr, len } => unsafe {
                slice::from_raw_parts_mut(ptr.as_ptr(), *len)
            },
        }
    }

    ///
    /// # Description
    ///
    /// Gets a slice to the underlying data in the backing storage.
    ///
    /// # Returns
    ///
    /// A slice to the underlying data in the backing storage.
    ///
    fn get(&self) -> &[T] {
        match self {
            RawArrayStorage::Managed { ptr, len } => unsafe {
                slice::from_raw_parts(ptr.as_ptr(), *len)
            },
            RawArrayStorage::Unmanaged { ptr, len } => unsafe {
                slice::from_raw_parts(ptr.as_ptr(), *len)
            },
        }
    }
}

// External type specification for RawArrayStorage.
#[verus_verify]
#[cfg(verus_keep_ghost)]
#[verus_verify(reject_recursive_types(T))]
#[verus_verify(external_type_specification)]
#[verus_verify(external_body)]
pub struct ExRawArrayStorage<T>(RawArrayStorage<T>);

//==================================================================================================
// Raw Array
//==================================================================================================

///
/// # Description
///
/// A type that represent a fixed-size array.
///
#[verus_verify]
#[verus_verify(reject_recursive_types(T))]
#[verus_verify(external_derive)]
#[derive(Debug)]
pub struct RawArray<T> {
    /// The backing storage of the raw array.
    storage: RawArrayStorage<T>,
}

#[verus_verify]
impl<T> RawArray<T> {
    ///
    /// # Description
    ///
    /// Constructs a new managed array.
    ///
    /// # Parameters
    ///
    /// - `len`: Length of the array.
    ///
    /// # Returns
    ///
    /// On success, the new managed array is returned, with all bits set to zero.
    /// On failure, an error is returned instead.
    ///
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        requires
            len > 0,
            len < i32::MAX as usize,
            len * vstd::layout::size_of::<T>() + vstd::layout::align_of::<T>() - 1
                <= isize::MAX as usize,
        ensures
            match result {
                Ok(me) => {
                    &&& me.inv()
                    &&& me@.len() == len
                    &&& forall|i: int| 0 <= i < len ==> is_zero(#[trigger] me@[i])
                },
                Err(e) => e.code == ErrorCode::OutOfMemory,
            },
    )]
    pub fn new(len: usize) -> Result<RawArray<T>, Error> {
        Ok(RawArray {
            storage: RawArrayStorage::new_managed(len)?,
        })
    }

    ///
    /// # Description
    ///
    /// Constructs a new unmanaged array.
    ///
    /// # Parameters
    ///
    /// - `ptr`: Pointer to the backing storage.
    /// - `len`: Length of the backing storage.
    ///
    /// # Returns
    ///
    /// On success, the new unmanaged array is returned, with all bits set to zero.
    /// On failure, an error is returned instead.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if any of the following conditions are violated:
    ///
    /// - `ptr` must be valid for both reads and writes for `len * mem::size_of::<T>()` many bytes.
    /// - `ptr` must be properly aligned.
    /// - `ptr` must point to len consecutive properly initialized values of type `T``.
    ///
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        requires
            len > 0,
            len < i32::MAX as usize,
            ptr.addr() != 0,
            ptr.addr() + len * vstd::layout::size_of::<T>() + vstd::layout::align_of::<T>() - 1
                <= usize::MAX as int,
        ensures
            match result {
                Ok(me) => {
                    &&& me.inv()
                    &&& me@.len() == len
                    &&& forall|i: int| 0 <= i < len ==> is_zero(#[trigger] me@[i])
                },
                Err(e) => e.code == ErrorCode::InvalidArgument,
            },
    )]
    pub unsafe fn from_raw_parts(ptr: *mut T, len: usize) -> Result<RawArray<T>, Error> {
        Ok(RawArray {
            storage: RawArrayStorage::new_unmanaged(ptr, len)?,
        })
    }

    /// Sets the element at index to value.
    /// Verus does not support mutable indexing (arr[i] = val), so this method
    /// provides a verified mutator with requires/ensures contracts.
    #[inline]
    #[verus_verify(external_body)]
    #[verus_spec(
        requires
            old(self).inv(),
            0 <= index < old(self)@.len(),
        ensures
            self.inv(),
            self@ == old(self)@.update(index as int, value),
    )]
    pub fn set(&mut self, index: usize, value: T) {
        self.storage.get_mut()[index] = value;
    }
}

#[verus_verify]
impl<T> Deref for RawArray<T> {
    type Target = [T];

    #[verus_verify(external_body)]
    #[verus_spec(result =>
        ensures
            result@ == self@,
    )]
    fn deref(&self) -> &Self::Target {
        self.storage.get()
    }
}

// Verus does not support &mut return types, so DerefMut is outside verus!{}.
impl<T> DerefMut for RawArray<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.storage.get_mut()
    }
}

impl<T> Drop for RawArray<T> {
    fn drop(&mut self) {
        match &self.storage {
            RawArrayStorage::Managed { ptr, len } => {
                let layout: Layout = match Layout::array::<T>(*len) {
                    Ok(layout) => layout,
                    Err(_) => return,
                };
                unsafe {
                    dealloc(ptr.as_ptr() as *mut u8, layout);
                }
            },
            RawArrayStorage::Unmanaged { .. } => (),
        }
    }
}
