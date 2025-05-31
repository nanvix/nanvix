// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A structure that represents an unaligned pointer.
///
pub struct UnalignedPointer<T> {
    ptr: *mut T,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl<T> UnalignedPointer<T> {
    ///
    /// # Description
    ///
    /// Creates a new unaligned pointer.
    ///
    /// # Parameters
    ///
    /// - `ptr`: The pointer to be unaligned.
    ///
    /// # Returns
    ///
    /// A new `UnalignedPointer`.
    ///
    pub fn new(ptr: *mut T) -> Self {
        UnalignedPointer { ptr }
    }
}

impl<T> UnalignedPointer<T> {
    ///
    /// # Description
    ///
    /// Reads the value at the unaligned pointer.
    ///
    /// # Returns
    ///
    /// The value pointed to by the unaligned pointer.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it dereferences a raw pointer.
    ///
    /// It is safe to call this function if and only if the following conditions are met:
    /// - `self` points to a valid memory location.
    ///
    pub unsafe fn read_unaligned(&self) -> T {
        unsafe { self.ptr.read_unaligned() }
    }

    ///
    /// # Description
    ///
    /// Writes a value to the unaligned pointer.
    ///
    /// # Parameters
    ///
    /// - `value`: The value to be written.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it dereferences a raw pointer.
    ///
    /// It is safe to call this function if and only if the following conditions are met:
    /// - `self` points to a valid memory location.
    ///
    pub unsafe fn write_unaligned(&mut self, value: T) {
        unsafe {
            self.ptr.write_unaligned(value);
        }
    }

    ///
    /// # Description
    ///
    /// Returns the raw pointer.
    ///
    /// # Returns
    ///
    /// The raw pointer.
    ///
    pub fn as_ptr(&self) -> *mut T {
        self.ptr
    }
}
