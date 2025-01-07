// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

use crate::Slab;
use ::sys::error::ErrorCode;

#[test]
fn test_slab_creation() {
    let mut memory = vec![0u32; 1024];
    let slab = unsafe { Slab::from_raw_parts(memory.as_mut_ptr() as *mut u8, memory.len(), 4) };
    assert!(slab.is_ok());
}

#[test]
fn test_slab_creation_invalid_length() {
    let mut memory = vec![0u32; 0];
    let slab = unsafe { Slab::from_raw_parts(memory.as_mut_ptr() as *mut u8, memory.len(), 4) };
    assert!(slab.is_err());
    assert_eq!(slab.unwrap_err().code, ErrorCode::InvalidArgument);
}

#[test]
fn test_slab_creation_invalid_block_size() {
    let mut memory = vec![0u32; 1024];
    let slab = unsafe { Slab::from_raw_parts(memory.as_mut_ptr() as *mut u8, memory.len(), 0) };
    assert!(slab.is_err());
    assert_eq!(slab.unwrap_err().code, ErrorCode::InvalidArgument);
}

#[test]
fn test_allocate_deallocate() {
    let mut memory = vec![0u32; 1024];
    let mut slab =
        unsafe { Slab::from_raw_parts(memory.as_mut_ptr() as *mut u8, memory.len(), 4) }.unwrap();

    let block = slab.allocate();
    assert!(block.is_ok());

    let dealloc_result = unsafe { slab.deallocate(block.unwrap()) };
    assert!(dealloc_result.is_ok());
}

#[test]
fn test_double_deallocate() {
    let mut memory = vec![0u32; 1024];
    let mut slab =
        unsafe { Slab::from_raw_parts(memory.as_mut_ptr() as *mut u8, memory.len(), 4) }.unwrap();

    let block = slab.allocate().unwrap();
    unsafe {
        slab.deallocate(block).unwrap();
    }

    let dealloc_result = unsafe { slab.deallocate(block) };
    assert!(dealloc_result.is_err());
    assert_eq!(dealloc_result.unwrap_err().code, ErrorCode::BadAddress);
}

#[test]
fn test_allocate_out_of_bounds() {
    let mut memory = vec![0u32; 1024];
    let mut slab =
        unsafe { Slab::from_raw_parts(memory.as_mut_ptr() as *mut u8, memory.len(), 4) }.unwrap();

    let invalid_ptr = unsafe { memory.as_mut_ptr().add(2048) };
    let dealloc_result = unsafe { slab.deallocate(invalid_ptr as *const u8) };
    assert!(dealloc_result.is_err());
    assert_eq!(dealloc_result.unwrap_err().code, ErrorCode::BadAddress);
}
