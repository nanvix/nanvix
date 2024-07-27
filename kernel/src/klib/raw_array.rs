// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::error::{
    Error,
    ErrorCode,
};
use ::core::{
    alloc::Layout,
    ops::{
        Deref,
        DerefMut,
    },
    ptr,
    slice,
};
use alloc::alloc;

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
            let ptr: *mut u8 = unsafe { alloc::alloc(layout) };
            match ptr::NonNull::new(ptr as *mut T) {
                Some(p) => p,
                None => {
                    error!("failed to allocate memory (layout={:?})", layout);
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

//==================================================================================================
// Raw Array
//==================================================================================================

///
/// # Description
///
/// A type that represent a fixed-size array.
///
#[derive(Debug)]
pub struct RawArray<T> {
    /// The backing storage of the raw array.
    storage: RawArrayStorage<T>,
}

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
    pub unsafe fn from_raw_parts(ptr: *mut T, len: usize) -> Result<RawArray<T>, Error> {
        Ok(RawArray {
            storage: RawArrayStorage::new_unmanaged(ptr, len)?,
        })
    }
}

impl<T> Deref for RawArray<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.storage.get()
    }
}

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
                    alloc::dealloc(ptr.as_ptr() as *mut u8, layout);
                }
            },
            RawArrayStorage::Unmanaged { .. } => (),
        }
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

/// Attempts to create a [`RawArray`] from a pointer and a length.
fn test_from_raw_parts() -> bool {
    let mut data: [u8; 4] = [1; 4];
    let ptr: *mut u8 = data.as_mut_ptr();
    let len: usize = data.len();
    let array = match unsafe { RawArray::from_raw_parts(ptr, len) } {
        Ok(array) => array,
        Err(_) => return false,
    };

    // Check if the array has the expected length.
    if array.len() != len {
        return false;
    }

    // Check if array was initialized with all bits set to zero.
    for i in array.iter() {
        if *i != 0 {
            return false;
        }
    }

    true
}

/// Attempts to create a [`RawArray`] with an invalid pointer.
fn test_from_raw_parts_invalid_ptr() -> bool {
    let ptr: *mut u8 = ptr::null_mut();
    let len: usize = 4;
    match unsafe { RawArray::from_raw_parts(ptr, len) } {
        Ok(_) => false,
        Err(e) if e.code == ErrorCode::InvalidArgument => true,
        _ => false,
    }
}

/// Attempts to create a [`RawArray`] with an invalid length.
fn test_from_raw_parts_invalid_len() -> bool {
    let mut data: [u8; 4] = [1; 4];
    let ptr: *mut u8 = data.as_mut_ptr();
    let len: usize = 0;
    match unsafe { RawArray::from_raw_parts(ptr, len) } {
        Ok(_) => false,
        Err(e) if e.code == ErrorCode::InvalidArgument => true,
        _ => false,
    }
}

/// Attempts to create a [`RawArray`] with a wrapping memory region.
fn test_from_raw_parts_wrapping_memory_region() -> bool {
    let ptr: *mut u8 = u32::MAX as *mut u8;
    let len: usize = 1;
    match unsafe { RawArray::from_raw_parts(ptr, len) } {
        Ok(_) => false,
        Err(e) if e.code == ErrorCode::InvalidArgument => true,
        _ => false,
    }
}

/// Runs all unit tests for [`RawArray`].
pub fn test_unmanaged() -> bool {
    let mut passed: bool = true;

    passed &= run_test!(test_from_raw_parts);
    passed &= run_test!(test_from_raw_parts_invalid_ptr);
    passed &= run_test!(test_from_raw_parts_invalid_len);
    passed &= run_test!(test_from_raw_parts_wrapping_memory_region);

    passed
}
