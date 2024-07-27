// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::klib::{
    bitmap::Bitmap,
    raw_array::RawArray,
};
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

        // Check if block size is valid.
        if block_size == 0 || block_size >= i32::MAX as usize || block_size > len {
            return Err(Error::new(ErrorCode::InvalidArgument, "invalid block size"));
        }

        // Check if the `block_size` is a power of two.
        if block_size & (block_size - 1) != 0 {
            return Err(Error::new(ErrorCode::InvalidArgument, "block size is not a power of two"));
        }

        // Check if `start_addr` is aligned to `block_size`.
        if (addr as usize) % block_size != 0 {
            return Err(Error::new(ErrorCode::InvalidArgument, "unaligned start address"));
        }

        // Compute layout of the slab allocator.
        let total_num_blocks: usize = len / block_size;
        // info!("total number of blocks: {:?}", total_num_blocks);
        if total_num_blocks % u8::BITS as usize != 0 {
            return Err(Error::new(ErrorCode::InvalidArgument, "invalid number of blocks"));
        }
        let index_len: usize = total_num_blocks / u8::BITS as usize;
        // info!("index length: {:?}", index_len);
        let num_index_blocks: usize =
            (index_len / block_size) + if index_len % block_size == 0 { 0 } else { 1 };
        // info!("number of index blocks: {:?}", num_index_blocks);
        let num_data_blocks: usize = total_num_blocks - num_index_blocks;
        // info!("number of data blocks: {:?}", num_data_blocks);
        let data_addr: *mut u8 = addr.add(num_index_blocks * block_size);

        // Check if `data_addr` is aligned to `block_size`.
        if (data_addr as usize) % block_size != 0 {
            return Err(Error::new(ErrorCode::InvalidArgument, "unaligned data address"));
        }

        // Instantiate index.
        let storage: RawArray<u8> = RawArray::from_raw_parts(addr, index_len)?;
        let mut index: Bitmap = Bitmap::from_raw_array(storage);

        // NOTE: The index is initialized with all blocks free, thus if we fail beyond this point
        // the memory region is left in a modified state.

        // Initialize index.
        for i in 0..num_index_blocks {
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
    pub fn deallocate(&mut self, ptr: *const u8) -> Result<(), Error> {
        // Check if the pointer lies in a memory region that is not managed by this allocator.
        // Safety: the start and resulting addresses are valid.
        if ptr < self.data_addr
            || ptr >= unsafe { self.data_addr.add(self.num_data_blocks * self.block_size) }
        {
            return Err(Error::new(ErrorCode::BadAddress, "pointer out of bounds"));
        }

        // Compute the block index.
        // Safety: we have already checked that ptr is within the bounds of the slab.
        let index: usize = unsafe { ptr.sub_ptr(self.data_addr) } / self.block_size;

        // Check if the block is already free.
        if !self.index.test(index)? {
            return Err(Error::new(ErrorCode::BadAddress, "block is already free"));
        }

        // Free the block.
        self.index.clear(index)?;

        Ok(())
    }
}
