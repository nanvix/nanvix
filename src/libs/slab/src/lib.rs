// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

//==================================================================================================
// Configuration
//==================================================================================================

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;

//==================================================================================================
// Imports
//==================================================================================================

use ::bitmap::Bitmap;
use ::raw_array::RawArray;
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A slab allocator.
///
/// It has the following layout in memory:
///
/// ```text
/// +-------------------+--------------------------------------+
/// | Index Blocks      | Data Blocks                          |
/// +-------------------+--------------------------------------+
/// ```
///
#[derive(Debug)]
pub struct Slab {
    /// An index that keeps track of free blocks.
    index: Bitmap,
    /// Base address of data blocks.
    data_addr: *mut u8,
    /// Number of data blocks in the slab.
    num_data_blocks: usize,
    /// Size of blocks in the slab.
    block_size: usize,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Slab {
    ///
    /// # Description
    ///
    /// Creates a new slab allocator on the memory region starting at `addr` with `len` bytes and
    /// block size of `block_size` bytes. The slab allocator is initialized with all blocks free.
    ///
    /// # Parameters
    ///
    /// - `addr`: Start address of the memory region.
    /// - `len`: Length of the memory region in bytes.
    /// - `block_size`: Size of blocks in bytes.
    ///
    /// # Returns
    ///
    /// Upon success, a new slab allocator is returned. Upon failure, an error is returned instead
    /// and the memory may be left in an modified state.
    ///
    /// # Safety
    ///
    /// This function is unsafe for the following reasons:
    /// - It assumes that the memory region starting at `addr` with `len` bytes is valid.
    ///
    pub unsafe fn from_raw_parts(
        addr: *mut u8,
        len: usize,
        block_size: usize,
    ) -> Result<Slab, Error> {
        // Check if length is invalid valid.
        if len == 0 || len >= i32::MAX as usize {
            return Err(Error::new(ErrorCode::InvalidArgument, "invalid slab length"));
        }

        // Check if the memory region wraps around.
        if addr.wrapping_add(len) < addr {
            return Err(Error::new(ErrorCode::InvalidArgument, "wrapping memory region"));
        }

        // Check if the block size is valid.
        if block_size == 0 || block_size >= i32::MAX as usize || block_size > len {
            return Err(Error::new(ErrorCode::InvalidArgument, "invalid block size"));
        }

        // Check if `addr` is aligned to `block_size`.
        if !(addr as usize).is_multiple_of(block_size) {
            return Err(Error::new(ErrorCode::InvalidArgument, "unaligned start address"));
        }

        // Compute layout of the slab allocator.
        let total_num_blocks: usize = len / block_size;

        // The number of index blocks (`num_index_blocks`) we need is
        //  `ceil(total_num_blocks / (block_size * u8::BITS + 1))`
        // for the following reason. This condition implies:
        //  `num_index_blocks * (block_size * u8::BITS + 1) >= total_num_blocks`
        // This, in turn, implies that:
        //  `num_index_blocks * block_size * u8::BITS  >= total_num_blocks - num_index_blocks`
        // The left-hand side of this inequality is the number of bits that
        // `num_index_blocks` blocks contain. The right-hand side of this inequality
        // is the number of blocks that aren't index blocks. So, a bitmap occupying
        // `num_index_blocks` blocks can address all the blocks outside of that bitmap.
        const U8_BITS: usize = u8::BITS as usize;
        let divisor: usize = block_size * U8_BITS + 1;
        let num_index_blocks: usize = (total_num_blocks / divisor)
            + if total_num_blocks.is_multiple_of(divisor) {
                0
            } else {
                1
            };
        if num_index_blocks >= total_num_blocks {
            return Err(Error::new(ErrorCode::InvalidArgument, "insufficient blocks for index"));
        }

        let data_addr: *mut u8 = addr.add(num_index_blocks * block_size);

        let num_data_blocks: usize = total_num_blocks - num_index_blocks;
        let index_len: usize = (num_data_blocks / U8_BITS)
            + if num_data_blocks.is_multiple_of(U8_BITS) {
                0
            } else {
                1
            };

        // Instantiate index.
        let storage: RawArray<u8> = RawArray::from_raw_parts(addr, index_len)?;
        let mut index: Bitmap = Bitmap::from_raw_array(storage)?;

        // NOTE: The index is initialized with all blocks free, thus if we fail beyond this point
        // the memory region is left in a modified state.

        // Initialize index.
        //
        // The uppermost bits of the index may point beyond the end of
        // the allocated region. So, we need to set those bits to mark
        // them "in use" and thereby prevent them from being
        // allocated. Note that there are at most 7 such bits we need
        // to set.
        for i in num_data_blocks..(index_len * U8_BITS) {
            index.set(i)?;
        }

        Ok(Slab {
            num_data_blocks,
            block_size,
            data_addr,
            index,
        })
    }

    ///
    /// # Description
    ///
    /// Allocates a block of memory from the slab allocator.
    ///
    /// # Returns
    ///
    /// Upon success, a pointer to the allocated block is returned. Upon failure, an error is
    /// returned instead.
    ///
    pub fn allocate(&mut self) -> Result<*mut u8, Error> {
        let block: usize = self.index.alloc()?;
        // Safety: the start and resulting addresses are valid.
        let block_addr: *mut u8 = unsafe { self.data_addr.add(block * self.block_size) };
        Ok(block_addr)
    }

    ///
    /// # Description
    ///
    /// Frees a block of memory from the slab allocator.
    ///
    /// # Parameters
    ///
    /// - `ptr`: Pointer to the block to free.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
    ///
    /// # Safety
    ///
    /// This function is unsafe for the following reasons:
    ///
    /// - It dereferences the pointer `ptr`.
    ///
    pub unsafe fn deallocate(&mut self, ptr: *const u8) -> Result<(), Error> {
        // Return an error if the pointer is before the data blocks.
        if ptr < self.data_addr {
            return Err(Error::new(ErrorCode::BadAddress, "pointer out of bounds"));
        }

        // Compute the block index.
        let index: usize = unsafe { ptr.offset_from_unsigned(self.data_addr) } / self.block_size;

        // Return an error if the pointer is after the data blocks.
        if index >= self.num_data_blocks {
            return Err(Error::new(ErrorCode::BadAddress, "pointer out of bounds"));
        }

        // Return an error if the block is already free.
        if !self.index.test(index)? {
            return Err(Error::new(ErrorCode::BadAddress, "block is already free"));
        }

        // Free the block.
        self.index.clear(index)?;

        Ok(())
    }
}
