// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::alloc::{
    alloc,
    dealloc,
};
use ::core::{
    alloc::Layout,
    mem::align_of,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    mm::{
        Address,
        VirtualAddress,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

/// Represents a user stack.
pub struct Stack {
    /// Base address of the stack.
    base: VirtualAddress,
    /// Size of the stack.
    size: usize,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Stack {
    ///
    /// # Description
    ///
    /// Creates a new user stack with the specified size. The stack is allocated in the user space
    /// and it is aligned to the size of a pointer.
    ///
    /// # Parameters
    ///
    /// - `size`: Size of the stack to be created.
    ///
    /// # Return Value
    ///
    /// On successful completion, this function returns the newly created stack. On failure, this
    /// function returns an error that contains the reason for the failure.
    ///
    /// # Errors
    ///
    /// This function fails with the following errors codes:
    /// - [`ErrorCode::OutOfMemory`]: The stack cannot be allocated due to insufficient memory.
    ///
    pub fn new(size: usize) -> Result<Self, Error> {
        ::syslog::trace!("new(): size={size:?}");

        // Compute allocation layout.
        // Align to pointer size to ensure the stack is properly aligned for any value stored.
        let layout: Layout = match Layout::from_size_align(size, align_of::<usize>()) {
            Ok(layout) => layout,
            Err(error) => {
                let reason: &str = "failed to create user stack layout";
                ::syslog::error!("new(): {reason:?} (error={error:?}, size={size:?})");
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            },
        };

        // Allocate memory for the stack and check for errors.
        let base: VirtualAddress = match unsafe { alloc(layout) } {
            ptr if !ptr.is_null() => VirtualAddress::from_raw_value(ptr as usize),
            _ => {
                let reason: &str = "failed to allocate user stack";
                ::syslog::error!("new(): {reason:?} (size={size:?})");
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            },
        };

        Ok(Self { base, size })
    }

    ///
    /// # Description
    ///
    /// Returns the base address of the stack.
    ///
    /// # Return Value
    ///
    /// The base address of the stack as a `VirtualAddress`.
    ///
    pub fn base(&self) -> VirtualAddress {
        self.base
    }

    ///
    /// # Description
    ///
    /// Returns the size of the stack.
    ///
    /// # Return Value
    ///
    /// The size of the stack in bytes as a `usize`.
    ///
    pub fn size(&self) -> usize {
        self.size
    }
}

impl Drop for Stack {
    fn drop(&mut self) {
        ::syslog::trace!("drop(): base={:#x}, size={:?}", self.base.into_raw_value(), self.size);

        // Compute allocation layout.
        let layout: Layout = match Layout::from_size_align(self.size, align_of::<usize>()) {
            Ok(layout) => layout,
            Err(error) => {
                ::syslog::error!(
                    "drop(): failed to create stack layout for drop (error={error:?})"
                );
                return;
            },
        };

        // Deallocate memory for the stack.
        // SAFETY: `self.base.into_raw_value()` returns the original pointer allocated by `alloc()`,
        // and thus is safe to cast to `*mut u8` for deallocation.
        unsafe {
            dealloc(self.base.into_raw_value() as *mut u8, layout);
        }
    }
}
