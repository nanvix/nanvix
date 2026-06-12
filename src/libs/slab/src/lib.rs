// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

//==================================================================================================
// Configuration
//==================================================================================================

#![cfg_attr(not(feature = "std"), no_std)]
// To support attributes on statements, e.g., #[verus_spec(invariant ...)] while ...,
// we need `proc_macro_hygiene`.
#![cfg_attr(verus_keep_ghost, feature(proc_macro_hygiene))]

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
use ::vstd::prelude::*;

// Include specifications.
#[cfg(verus_keep_ghost)]
include!("lib.spec.rs");
// Include proofs.
#[cfg(verus_keep_ghost)]
include!("lib.proof.rs");

//==================================================================================================
// Constants
//==================================================================================================

/// Slab poison byte used in debug builds to fill freed blocks and make use-after-free bugs more
/// likely to cause a crash and easier to diagnose. This is not used in release builds to avoid
/// the performance overhead of filling freed blocks.
#[cfg(all(debug_assertions, not(verus_keep_ghost)))]
pub const SLAB_POISON_BYTE: u8 = 0xDE;

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
#[verus_verify(external_derive)]
#[derive(Debug)]
pub struct Slab {
    /// An index that keeps track of free blocks.
    index: Bitmap,
    /// Base address of data blocks.
    data_addr: *mut u8,
    /// End of data blocks.
    end_addr: *const u8,
    /// Size of blocks in the slab.
    block_size: usize,
}

//==================================================================================================
// Implementations
//==================================================================================================

#[verus_verify]
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
    #[verus_spec(result =>
         ensures
             match result {
                 Ok(slab) => {
                     &&& slab.inv()
                     &&& slab@.block_size == block_size
                     &&& slab@.start_addr >= addr as usize
                     &&& slab@.end_addr <= addr as usize + len
                     &&& slab@.allocated_addrs == Set::<usize>::empty()
                     &&& forall|i: int| 0 <= i < (slab@.end_addr - slab@.start_addr) / block_size as int
                         ==> #[trigger] slab@.free_addrs.contains(
                             (slab@.start_addr + i * block_size as int) as usize)
                 },
                 Err(e) => {
                     &&& e.code == ErrorCode::InvalidArgument
                     &&& {
                         ||| addr as usize == 0
                         ||| len == 0
                         ||| len >= i32::MAX
                         ||| len > isize::MAX
                         ||| addr as usize + len > usize::MAX
                         ||| block_size == 0
                         ||| block_size >= i32::MAX
                         ||| block_size > (usize::MAX - 1) / (u8::BITS as int)
                         ||| len < block_size * 2
                         ||| addr as usize % block_size != 0
                     }
                 }
             },
    )]
    pub unsafe fn from_raw_parts(
        addr: *mut u8,
        len: usize,
        block_size: usize,
    ) -> Result<Slab, Error> {
        // Make sure the address isn't null, e.g., from a failed allocation.
        if addr.is_null() {
            return Err(Error::new(ErrorCode::InvalidArgument, "null pointer"));
        }

        // Check if length is invalid.
        if len == 0 || len >= i32::MAX as usize || len > isize::MAX as usize {
            return Err(Error::new(ErrorCode::InvalidArgument, "invalid slab length"));
        }

        // Check if the memory region wraps around.
        proof! { Self::lemma_wrapping_add_consequences(addr, len); }
        if addr.wrapping_add(len) < addr {
            return Err(Error::new(ErrorCode::InvalidArgument, "wrapping memory region"));
        }

        // Check if the block size is valid.
        // TODO: Make this `const U8_BITS` instead of `let u8_bits` once issue
        // https://github.com/verus-lang/verus/issues/2023 is fixed.
        let u8_bits: usize = u8::BITS as usize;
        if block_size == 0
            || block_size >= i32::MAX as usize
            || block_size > (usize::MAX - 1) / u8_bits
            || block_size > len
        {
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
        let divisor: usize = block_size * u8_bits + 1;
        let num_index_blocks: usize = (total_num_blocks / divisor)
            + if total_num_blocks.is_multiple_of(divisor) {
                0
            } else {
                1
            };
        if num_index_blocks >= total_num_blocks {
            proof! {
                Self::lemma_no_room_for_index(len, block_size, total_num_blocks,
                                              num_index_blocks, divisor);
            }
            return Err(Error::new(ErrorCode::InvalidArgument, "insufficient blocks for index"));
        }

        proof! {
            Slab::lemma_can_compute_data_addr(addr, total_num_blocks, num_index_blocks,
                                              block_size, len);
        }
        let data_addr: *mut u8 = addr.add(num_index_blocks * block_size);

        let num_data_blocks: usize = total_num_blocks - num_index_blocks;
        let index_len: usize = (num_data_blocks / u8_bits)
            + if num_data_blocks.is_multiple_of(u8_bits) {
                0
            } else {
                1
            };

        // Instantiate index.
        proof! {
            Slab::lemma_can_create_raw_array(addr, total_num_blocks, num_index_blocks,
                                             num_data_blocks, block_size, len, index_len);
        }
        let storage: RawArray<u8> = RawArray::from_raw_parts(addr, index_len)?;

        proof! {
            assert forall|i| 0 <= i < index_len implies storage@[i] == 0 by {
                raw_array::axiom_u8_zero_is_0(storage@[i]);
            }
        }
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
        #[cfg_attr(verus_keep_ghost, verus_spec(
            invariant
                index.inv(),
                index@.num_bits == index_len * u8_bits,
                index@.set_bits == Set::range(num_data_blocks as int, i as int),
        ))]
        for i in num_data_blocks..(index_len * u8_bits) {
            index.set(i)?;
        }

        let end_addr = addr.add(total_num_blocks * block_size);
        proof! {
            Slab::lemma_from_raw_parts_establishes_inv(
                block_size, data_addr, end_addr, &index,
                addr, len, total_num_blocks, num_index_blocks,
                num_data_blocks, index_len, u8_bits,
            );
        }
        Ok(Slab {
            index,
            data_addr,
            end_addr,
            block_size,
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
    #[verus_spec(result =>
        requires
            old(self).inv(),
        ensures
            final(self).inv(),
            match result {
                Ok(ptr) => {
                    let addr = ptr as usize;
                    &&& old(self)@.free_addrs.contains(addr)
                    &&& addr % final(self)@.block_size == 0
                    &&& final(self)@ == SlabView {
                        allocated_addrs: old(self)@.allocated_addrs.insert(addr),
                        free_addrs: old(self)@.free_addrs.remove(addr),
                        ..old(self)@
                    }
                },
                Err(_) => {
                    &&& old(self)@.free_addrs == Set::<usize>::empty()
                    &&& final(self)@ == old(self)@
                },
            },
    )]
    pub fn allocate(&mut self) -> Result<*mut u8, Error> {
        let block: usize = self.index.alloc()?;

        proof! { self.lemma_allocate_add_is_safe(block); }
        let block_addr: *mut u8 = unsafe { self.data_addr.add(block * self.block_size) };

        proof! { self.lemma_allocate_ok(old(self), block, block_addr as usize); }

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
    /// - It uses `offset_from_unsigned`.
    ///
    #[verus_spec(result =>
        requires
            old(self).inv(),
        ensures
            final(self).inv(),
            match result {
                Ok(()) => {
                    &&& old(self)@.allocated_addrs.contains(ptr as usize)
                    &&& final(self)@ == (SlabView {
                        allocated_addrs: old(self)@.allocated_addrs.remove(ptr as usize),
                        free_addrs: old(self)@.free_addrs.insert(ptr as usize),
                        ..old(self)@
                    })
                },
                Err(_) => {
                    &&& !old(self)@.allocated_addrs.contains(ptr as usize)
                    &&& final(self)@ == old(self)@
                },
            },
    )]
    pub unsafe fn deallocate(&mut self, ptr: *const u8) -> Result<(), Error> {
        // Return an error if the pointer is before or after the data blocks.
        if ptr < self.data_addr as *const u8 || ptr >= self.end_addr {
            return Err(Error::new(ErrorCode::BadAddress, "pointer out of bounds"));
        }

        // Return an error if the pointer isn't at a block boundary.
        if !(ptr as usize).is_multiple_of(self.block_size) {
            return Err(Error::new(ErrorCode::BadAddress, "pointer unaligned"));
        }

        proof! { self.lemma_deallocate_offset_bound(ptr); }

        // Compute the block index.
        let index: usize = unsafe { ptr.offset_from_unsigned(self.data_addr) } / self.block_size;

        proof! { self.lemma_deallocate_index_ok(ptr, index); }

        // Return an error if the block is already free.
        if !self.index.test(index)? {
            return Err(Error::new(ErrorCode::BadAddress, "block is already free"));
        }

        // Free the block.
        self.index.clear(index)?;

        // Poison the freed block so that any use-after-free dereference through a stale pointer
        // reads a recognizable garbage pattern instead of silently reusing stale data.
        #[cfg(all(debug_assertions, not(verus_keep_ghost)))]
        unsafe {
            core::ptr::write_bytes(ptr as *mut u8, SLAB_POISON_BYTE, self.block_size);
        }

        proof! { self.lemma_deallocate_ok(old(self), index, ptr); }

        Ok(())
    }
}
