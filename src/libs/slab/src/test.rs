// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

use crate::Slab;
use ::sys::error::ErrorCode;

#[test]
fn test_slab_creation() {
    let mut memory = vec![0u8; 1024];
    let slab = unsafe { Slab::from_raw_parts(memory.as_mut_ptr(), memory.len(), 4) };
    assert!(slab.is_ok());
}

#[test]
fn test_slab_creation_invalid_length() {
    let mut memory = vec![0u8; 0];
    let slab = unsafe { Slab::from_raw_parts(memory.as_mut_ptr(), memory.len(), 4) };
    assert!(slab.is_err());
    assert_eq!(slab.unwrap_err().code, ErrorCode::InvalidArgument);
}

#[test]
fn test_slab_creation_invalid_block_size() {
    let mut memory = vec![0u8; 1024];
    let slab = unsafe { Slab::from_raw_parts(memory.as_mut_ptr(), memory.len(), 0) };
    assert!(slab.is_err());
    assert_eq!(slab.unwrap_err().code, ErrorCode::InvalidArgument);
}

#[test]
fn test_allocate_deallocate() {
    let mut memory = vec![0u8; 1024];
    let mut slab = unsafe { Slab::from_raw_parts(memory.as_mut_ptr(), memory.len(), 4) }.unwrap();

    let block = slab.allocate();
    assert!(block.is_ok());

    let dealloc_result = unsafe { slab.deallocate(block.unwrap()) };
    assert!(dealloc_result.is_ok());
}

#[test]
fn test_double_deallocate() {
    let mut memory = vec![0u8; 1024];
    let mut slab = unsafe { Slab::from_raw_parts(memory.as_mut_ptr(), memory.len(), 4) }.unwrap();

    let block = slab.allocate().unwrap();
    unsafe {
        slab.deallocate(block).unwrap();
    }

    let dealloc_result = unsafe { slab.deallocate(block) };
    assert!(dealloc_result.is_err());
    assert_eq!(dealloc_result.unwrap_err().code, ErrorCode::BadAddress);
}

#[test]
fn test_slab_creation_minimal_valid_blocks() {
    // Use a minimal valid configuration: 2 blocks of block_size=1 bytes.
    // This exercises the smallest valid slab layout and ensures the
    // guard against `num_index_blocks >= total_num_blocks` does not
    // reject a valid configuration.
    let block_size: usize = 1;
    let total_num_blocks: usize = 2;
    let len: usize = total_num_blocks * block_size;
    let mut memory = vec![0u8; len];
    let slab = unsafe { Slab::from_raw_parts(memory.as_mut_ptr(), len, block_size) };
    // With block_size=1 and len=2: total_num_blocks=2, num_index_blocks=1 → should succeed.
    assert!(slab.is_ok());
    let mut slab = slab.unwrap();

    // It should only be possible to allocate a single block from this
    // minimal configuration: the one block that isn't an index block.
    // The failure to allocate more than one block tests that the
    // current implementation correctly avoids allocating beyond the
    // end of the valid region.
    let block1 = slab.allocate();
    assert!(block1.is_ok());
    let block1 = block1.unwrap();
    assert!(memory.as_ptr() <= block1);
    assert!(unsafe { block1.add(block_size) <= memory.as_mut_ptr().add(len) });
    let block2 = slab.allocate();
    assert!(block2.is_err());
}

#[test]
fn test_slab_creation_non_power_of_2() {
    let block_size: usize = 10;
    let total_num_blocks: usize = 15;
    let len: usize = total_num_blocks * block_size;
    let mut memory = vec![0u8; len + block_size];
    let memory_offset_mod_block_size: usize = (memory.as_mut_ptr() as usize) % block_size;
    let padding_needed = if memory_offset_mod_block_size != 0 {
        block_size - memory_offset_mod_block_size
    } else {
        0
    };
    let slab =
        unsafe { Slab::from_raw_parts(memory.as_mut_ptr().add(padding_needed), len, block_size) };
    assert!(slab.is_ok());
}

#[test]
fn test_allocate_out_of_bounds() {
    let mut memory = vec![0u8; 1024];
    let mut slab = unsafe { Slab::from_raw_parts(memory.as_mut_ptr(), memory.len(), 4) }.unwrap();

    let invalid_ptr = unsafe { memory.as_mut_ptr().add(2048) };
    let dealloc_result = unsafe { slab.deallocate(invalid_ptr as *const u8) };
    assert!(dealloc_result.is_err());
    assert_eq!(dealloc_result.unwrap_err().code, ErrorCode::BadAddress);
}
