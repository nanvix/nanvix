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
    mem::size_of,
    ptr::{
        copy_nonoverlapping,
        null_mut,
    },
};
use ::syslog::{
    error,
    warn,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Alignment for allocations passed to the underlying allocator. Must be at
/// least `align_of::<LlistNode>()` (8 on 32-bit) so that talc can safely write
/// its free-list nodes into freed chunks without misaligned access.
const UNDERLYING_ALIGNMENT: usize = 8;
/// Alignment of the BlockHeader struct itself.
const BLOCK_HEADER_ALIGNMENT: usize = core::mem::align_of::<BlockHeader>();
/// Block header size.
const BLOCK_HEADER_SIZE: usize = size_of::<BlockHeader>();
/// Maximum alignment that the allocator supports.
const MAX_ALIGNMENT: usize = 4096;
/// Start of user mmap region.
const USER_MMAP_BASE_RAW: usize = ::config::memory_layout::USER_MMAP_BASE_RAW;
/// End of user mmap region.
const USER_MMAP_END_RAW: usize = ::config::memory_layout::USER_MMAP_END_RAW;

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
        // Assert pre-conditions.
        debug_assert!(size > 0, "alloc(): zero-size allocation");
        #[cfg(debug_assertions)]
        if let Some(align) = alignment {
            debug_assert!(align > 0, "alloc(): zero-size alignment");
        }

        // Get alignment for user memory area, or default to minimum alignment.
        let alignment: usize = alignment.unwrap_or(1);

        // Validate alignment invariants so that free() can trust the header.
        if !alignment.is_power_of_two() || alignment > MAX_ALIGNMENT {
            error!("alloc(): invalid alignment (alignment={alignment:?}, size={size:?})");
            return null_mut();
        }

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

            // Round up to UNDERLYING_ALIGNMENT so that talc's free-list nodes
            // (LlistNode, 8 bytes) are always naturally aligned when placed at
            // the base of a freed chunk.
            let Some(rounded) = alloc_size.checked_add(UNDERLYING_ALIGNMENT - 1) else {
                error!(
                    "alloc(): overflow when rounding allocation size (size={size:?}, \
                     alignment={alignment:?})"
                );
                return null_mut();
            };
            rounded & !(UNDERLYING_ALIGNMENT - 1)
        };

        // Compute layout for underlying allocation.
        let layout: Layout = match Layout::from_size_align(alloc_size, UNDERLYING_ALIGNMENT) {
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
        // Assert pre-conditions.
        debug_assert!(!user_ptr.is_null(), "free(): null user pointer");

        let header_ptr: *mut BlockHeader = Self::get_mut_ptr(user_ptr);
        let header: BlockHeader = header_ptr.read(); // move out

        // Validate header sanity.
        // After a shrinking realloc, requested_alloc_size < original, so we only check >=.
        let valid_alignment: bool = header.alignment > 0
            && header.alignment.is_power_of_two()
            && header.alignment <= MAX_ALIGNMENT;
        let valid_alloc_size: bool = header
            .requested_alloc_size
            .checked_add(BLOCK_HEADER_SIZE)
            .is_some_and(|min_size| header.alloc_size >= min_size);
        let base_addr: usize = header.base as usize;
        let valid_base: bool = (USER_MMAP_BASE_RAW..USER_MMAP_END_RAW).contains(&base_addr);
        let uptr: usize = user_ptr as usize;
        let valid_user_ptr: bool = base_addr
            .checked_add(header.alloc_size)
            .is_some_and(|end| uptr >= base_addr && uptr < end && end <= USER_MMAP_END_RAW);
        let valid_underlying_alignment: bool = base_addr.is_multiple_of(UNDERLYING_ALIGNMENT)
            && header.alloc_size.is_multiple_of(UNDERLYING_ALIGNMENT);
        let valid: bool = valid_alignment
            && valid_alloc_size
            && valid_base
            && valid_user_ptr
            && valid_underlying_alignment;

        if !valid {
            error!(
                "BlockHeader::free(): corrupted header detected, leaking (user_ptr={user_ptr:p}, \
                 header={header:?})"
            );
            return Err(());
        }

        let layout: Layout = match Layout::from_size_align(header.alloc_size, UNDERLYING_ALIGNMENT)
        {
            Ok(layout) => layout,
            Err(error) => {
                error!(
                    "BlockHeader::free(): corrupted header (error={error:?}, header={header:?})"
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

    ///
    /// # Description
    ///
    /// Returns the usable (requested) size of an allocated block given a user pointer.
    ///
    /// # Parameters
    ///
    /// - `user_ptr`: Pointer previously returned to the caller by an allocation function.
    ///
    /// # Returns
    ///
    /// This function returns the number of bytes that were originally requested for the
    /// allocation (the logical usable size). If the caller requested a reallocation that shrunk
    /// the block, the returned value reflects the new (smaller) size. Behavior is undefined if
    /// `user_ptr` was not allocated by this allocator.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `user_ptr` originates from a successful call to one of the
    /// allocation routines backed by this allocator and that it has not been freed yet.
    ///
    pub(crate) unsafe fn usable_size(user_ptr: *mut u8) -> usize {
        debug_assert!(!user_ptr.is_null(), "usable_size(): null user pointer");
        let header: &mut BlockHeader = Self::get_mut_ref(user_ptr);
        header.requested_alloc_size
    }
}
