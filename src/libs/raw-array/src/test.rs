// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    collections::raw_array::RawArray,
    error::ErrorCode,
};
use ::core::ptr;

//==================================================================================================
// Unit Tests
//==================================================================================================

/// Attempts to create a [`RawArray`] from a pointer and a length.
#[test]
fn test_from_raw_parts() {
    let mut data: [u8; 4] = [1; 4];
    let ptr: *mut u8 = data.as_mut_ptr();
    let len: usize = data.len();
    let array = match unsafe { RawArray::from_raw_parts(ptr, len) } {
        Ok(array) => array,
        Err(e) => panic!("failed to create array from raw parts (error={:?})", e),
    };

    // Check if the array has the expected length.
    if array.len() != len {
        panic!("array has unexpected length (expected={}, got={})", len, array.len());
    }

    // Check if array was initialized with all bits set to zero.
    for i in array.iter() {
        if *i != 0 {
            panic!("array was not initialized with all bits set to zero");
        }
    }
}

/// Attempts to create a [`RawArray`] with an invalid pointer.
#[test]
fn test_from_raw_parts_invalid_ptr() {
    let ptr: *mut u8 = ptr::null_mut();
    let len: usize = 4;
    match unsafe { RawArray::from_raw_parts(ptr, len) } {
        Ok(_) => panic!("created array with invalid pointer"),
        Err(e) if e.code == ErrorCode::InvalidArgument => {},
        Err(e) => panic!("unexpected error code (error={:?})", e),
    }
}

/// Attempts to create a [`RawArray`] with an invalid length.
#[test]
fn test_from_raw_parts_invalid_len() {
    let mut data: [u8; 4] = [1; 4];
    let ptr: *mut u8 = data.as_mut_ptr();
    let len: usize = 0;
    match unsafe { RawArray::from_raw_parts(ptr, len) } {
        Ok(_) => panic!("created array with invalid length"),
        Err(e) if e.code == ErrorCode::InvalidArgument => {},
        Err(e) => panic!("unexpected error code (error={:?})", e),
    }
}

/// Attempts to create a [`RawArray`] with a wrapping memory region.
#[test]
fn test_from_raw_parts_wrapping_memory_region() {
    let ptr: *mut u8 = usize::MAX as *mut u8;
    let len: usize = 1;
    match unsafe { RawArray::from_raw_parts(ptr, len) } {
        Ok(_) => panic!("created array with wrapping memory region"),
        Err(e) if e.code == ErrorCode::InvalidArgument => {},
        Err(e) => panic!("unexpected error code (error={:?})", e),
    }
}
