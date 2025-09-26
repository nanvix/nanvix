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
    mem::{
        align_of,
        size_of,
    },
    ptr::{
        copy_nonoverlapping,
        null_mut,
    },
};
use ::syslog::{
    error,
    trace,
    warn,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Minimum alignment guaranteed by this allocator.
const BLOCK_HEADER_ALIGNMENT: usize = align_of::<usize>();
/// Block header size.
const BLOCK_HEADER_SIZE: usize = size_of::<BlockHeader>();

//==================================================================================================
// Structures
//==================================================================================================

/// Allocation metadata stored immediately before the user pointer.
#[derive(Debug)]
#[repr(C)]
pub(crate) struct BlockHeader {
    /// Pointer returned by the underlying global allocator (base block start).
    base: *mut u8,
    /// Size passed to the underlying allocator (layout.size()).
    alloc_size: usize,
    /// Requested (logical) user size.
    requested_alloc_size: usize,
    /// Allocation alignment.
    alignment: usize,
}

//==================================================================================================
// Helpers
//==================================================================================================

impl BlockHeader {
    /// # Description
    ///
    /// Allocates a block of memory.
    ///
    /// This function allocates a block of memory of `size` bytes with the specified `alignment`. If
    /// `alignment` is `None`, a byte-aligned block is allocated.
    ///
    /// # Parameters
    ///
    /// - `size`: Size in bytes.
    /// - `alignment`: Optional alignment in bytes.
    ///
    /// # Returns
    ///
    /// On success, this function returns a pointer to the allocated memory. On failure, it returns
    /// a null pointer.
    ///
    /// # Safety
    ///
    ///  This function is unsafe because it interacts with the global memory allocator.
    ///
    pub(crate) unsafe fn alloc(size: usize, alignment: Option<usize>) -> *mut u8 {
        trace!("alloc(): size={size:?}, alignment={alignment:?}");

        // Assert pre-conditions.
        debug_assert!(size > 0, "alloc(): zero-size allocation");
        #[cfg(debug_assertions)]
        if let Some(align) = alignment {
            debug_assert!(align > 0, "alloc(): zero-size alignment");
        }

        // Get alignment for user memory area, or default to minimum alignment.
        let alignment: usize = alignment.unwrap_or(1);

        // Compute size for underlying allocation (header_size + padding_size + requested_size).
        let alloc_size: usize = {
            let Some(alloc_size) = size.checked_add(BLOCK_HEADER_SIZE) else {
                error!(
                    "alloc(): overflow when computing allocation size (size={size:?}, \
                     alignment={alignment:?})"
                );
                return null_mut();
            };
            let Some(alloc_size) = alloc_size.checked_add(alignment - 1) else {
                error!(
                    "alloc(): overflow when computing allocation size (size={size:?}, \
                     alignment={alignment:?})"
                );
                return null_mut();
            };

            alloc_size
        };

        // Compute layout for underlying allocation.
        let layout: Layout = match Layout::from_size_align(alloc_size, BLOCK_HEADER_ALIGNMENT) {
            Ok(layout) => layout,
            Err(error) => {
                error!("alloc(): {error:?} (alignment={alignment:?}, size={size:?})");
                return null_mut();
            },
        };

        // Perform allocation and check for errors.
        let base: *mut u8 = alloc(layout);
        if base.is_null() {
            error!(
                "alloc(): underlying allocation failed (alignment={alignment:?}, size={size:?})"
            );
            return null_mut();
        }

        // Create block header, write it to memory, and return user pointer.
        let block_header: BlockHeader = BlockHeader {
            base,
            alloc_size,
            requested_alloc_size: size,
            alignment,
        };
        block_header.write()
    }

    ///
    /// # Description
    ///
    /// Reallocates a block of memory.
    ///
    /// # Parameters
    ///
    /// - `user_ptr`: Pointer to the user memory area.
    /// - `new_size`: New size in bytes.
    ///
    /// # Returns
    ///
    /// On success, this function returns a pointer to the reallocated memory. On failure, it
    /// returns a null pointer.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it interacts with the global memory allocator.
    ///
    pub(crate) unsafe fn realloc(user_ptr: *mut u8, new_size: usize) -> *mut u8 {
        trace!("realloc(): user_ptr={user_ptr:?}, new_size={new_size:?}");

        // Assert pre-conditions.
        debug_assert!(!user_ptr.is_null(), "realloc(): null user pointer");
        debug_assert!(new_size > 0, "realloc(): zero-size reallocation");

        let header_ref: &mut BlockHeader = BlockHeader::get_mut_ref(user_ptr);
        let old_size: usize = header_ref.requested_alloc_size;

        // If shrinking or same size, keep allocation.
        if new_size <= old_size {
            header_ref.requested_alloc_size = new_size;
            return user_ptr;
        }

        // Allocate new block and check for errors.
        let new_ptr: *mut u8 = BlockHeader::alloc(new_size, Some(header_ref.alignment));
        if new_ptr.is_null() {
            error!("realloc(): allocation failed (user_ptr={user_ptr:?}, new_size={new_size:?})");
            return null_mut();
        }

        // Copy old data to new block.
        copy_nonoverlapping(user_ptr, new_ptr, old_size);

        // Free old block and check for errors.
        if BlockHeader::free(user_ptr).is_err() {
            warn!("realloc(): failed to free old block, leaking memory (user_ptr={user_ptr:?})");
        }

        new_ptr
    }

    ///
    /// # Description
    ///
    /// Frees a block of memory.
    ///
    /// # Parameters
    ///
    /// - `user_ptr`: Pointer to the user memory area.
    ///
    /// # Returns
    ///
    /// On success, this function returns `Ok(())`. On failure, it returns `Err(())`.
    ///
    pub(crate) unsafe fn free(user_ptr: *mut u8) -> Result<(), ()> {
        trace!("free(): user_ptr={user_ptr:?}");

        // Assert pre-conditions.
        debug_assert!(!user_ptr.is_null(), "free(): null user pointer");

        let header_ptr: *mut BlockHeader = Self::get_mut_ptr(user_ptr);
        let header: BlockHeader = header_ptr.read(); // move out
        let layout: Layout =
            match Layout::from_size_align(header.alloc_size, BLOCK_HEADER_ALIGNMENT) {
                Ok(layout) => layout,
                Err(error) => {
                    // Corrupted header; cannot recover.
                    error!(
                        "BlockHeader::free(): corrupted header (error={error:?}, \
                         header={header:?})"
                    );
                    return Err(());
                },
            };

        dealloc(header.base, layout);

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Writes the block header to memory and returns the user pointer.
    ///
    /// # Parameters
    ///
    /// - `user_ptr`: Pointer to the user memory area.
    ///
    /// # Returns
    ///
    /// This function returns the pointer to the user memory area.
    ///
    unsafe fn write(self) -> *mut u8 {
        // Align user pointer.
        let user_ptr_addr: usize = {
            let unaligned_user_ptr_addr: usize = self.base as usize + BLOCK_HEADER_SIZE;
            let rem: usize = unaligned_user_ptr_addr % self.alignment;
            if rem == 0 {
                unaligned_user_ptr_addr
            } else {
                unaligned_user_ptr_addr + (self.alignment - rem)
            }
        };
        let header_ptr: *mut BlockHeader = Self::get_mut_ptr(user_ptr_addr as *mut u8);
        header_ptr.write(self);

        user_ptr_addr as *mut u8
    }

    ///
    /// # Description
    ///
    /// Gets a mutable reference to the block header from the user pointer.
    ///
    /// # Parameters
    ///
    /// - `user_ptr`: Pointer to the user memory area.
    ///
    /// # Returns
    ///
    /// This function returns a mutable reference to the block header.
    ///
    unsafe fn get_mut_ref<'a>(user_ptr: *mut u8) -> &'a mut BlockHeader {
        let header_ptr: *mut BlockHeader = Self::get_mut_ptr(user_ptr);
        &mut *header_ptr
    }

    ///
    /// # Description
    ///
    /// Gets a mutable pointer to the block header from the user pointer.
    ///
    /// # Parameters
    ///
    /// - `user_ptr`: Pointer to the user memory area.
    ///
    /// # Returns
    ///
    /// This function returns a mutable pointer to the block header.
    ///
    #[inline(always)]
    unsafe fn get_mut_ptr(user_ptr: *mut u8) -> *mut BlockHeader {
        let unaligned_header_addr: usize = user_ptr as usize - BLOCK_HEADER_SIZE;
        let aligned_header_addr: usize = unaligned_header_addr & !(BLOCK_HEADER_ALIGNMENT - 1);
        aligned_header_addr as *mut BlockHeader
    }
}
