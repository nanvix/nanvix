// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::common::{
    StressError,
    exposed_addr_to_mut_u8,
};
use ::config::constants::KILOBYTE;
use ::core::convert::TryFrom;
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
use ::sysapi::sys_mman::prot_flags;
use ::syscall::sys::mman::{
    MemoryMapProtectionFlags,
    mmap,
    munmap,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Number of rapid map/unmap cycles.
const CYCLES: usize = 32;

/// Sizes to cycle through (all page-aligned).
const MAP_SIZES: [usize; 4] = [4 * KILOBYTE, 8 * KILOBYTE, 16 * KILOBYTE, 64 * KILOBYTE];

/// Number of concurrently held mappings before a batch unmap.
const BATCH_SIZE: usize = 4;

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Rapidly maps and unmaps memory regions of varying sizes through the unified mmap syscall API.
/// This exercises the virtual address bump allocator and the segment tracking map under load.
///
/// The test proceeds in cycles. Each cycle maps `BATCH_SIZE` regions of pseudo-random sizes,
/// writes a tag byte to each, then unmaps all of them. Additionally, some cycles perform
/// single map/write/unmap operations to test the shortest allocation lifetime.
///
/// # Returns
///
/// `Ok(())` on success or an error if any mapping, write, or unmapping operation fails.
///
pub fn run() -> Result<(), StressError> {
    let mut rng: u32 = 0xDEAD_C0DE;

    for cycle in 0..CYCLES {
        // Phase 1: Batch map.
        let mut mappings: [(VirtualAddress, usize); BATCH_SIZE] =
            [(VirtualAddress::from_raw_value(0), 0); BATCH_SIZE];

        for (slot, mapping) in mappings.iter_mut().enumerate() {
            rng = super::common::xorshift32(rng);
            let size_index: usize = usize::try_from(rng).unwrap_or(0) % MAP_SIZES.len();
            let size: usize = MAP_SIZES[size_index];

            let prot: MemoryMapProtectionFlags =
                MemoryMapProtectionFlags::try_from(prot_flags::PROT_READ | prot_flags::PROT_WRITE)?;

            let base: VirtualAddress = mmap(size, prot)?;

            // Write a tag to verify the mapping is writable.
            let tag: u8 = u8::try_from((cycle * BATCH_SIZE + slot) & 0xFF).unwrap_or(0);
            // SAFETY: `base` points to a freshly mmap'd region of `size` bytes with
            // PROT_READ | PROT_WRITE. We write and read a single byte at the start,
            // which is within the mapped region's bounds.
            unsafe {
                let ptr: *mut u8 = exposed_addr_to_mut_u8(base.into_raw_value());
                ptr.write_volatile(tag);
                let readback: u8 = ptr.read_volatile();
                if readback != tag {
                    return Err(Error::new(
                        ErrorCode::InvalidArgument,
                        "mmap data integrity check failed",
                    ));
                }
            }

            *mapping = (base, size);
        }

        // Phase 2: Batch unmap.
        for (base, size) in mappings.iter() {
            if *size > 0 {
                munmap(*base, *size)?;
            }
        }

        // Phase 3: Single-shot map/write/unmap for the shortest lifetime.
        {
            rng = super::common::xorshift32(rng);
            let size: usize = MAP_SIZES[usize::try_from(rng).unwrap_or(0) % MAP_SIZES.len()];
            let prot: MemoryMapProtectionFlags =
                MemoryMapProtectionFlags::try_from(prot_flags::PROT_READ | prot_flags::PROT_WRITE)?;

            let base: VirtualAddress = mmap(size, prot)?;

            // SAFETY: `base` points to a freshly mmap'd region of `size` bytes with
            // PROT_READ | PROT_WRITE. We write and read a single byte at the start,
            // which is within the mapped region's bounds.
            unsafe {
                let ptr: *mut u8 = exposed_addr_to_mut_u8(base.into_raw_value());
                ptr.write_volatile(0xAB);
                let readback: u8 = ptr.read_volatile();
                if readback != 0xAB {
                    return Err(Error::new(
                        ErrorCode::InvalidArgument,
                        "single-shot mmap data integrity check failed",
                    ));
                }
            }

            munmap(base, size)?;
        }
    }

    Ok(())
}
