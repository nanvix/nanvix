// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    collections::{
        raw_array::RawArray,
        Bitmap,
    },
    error::Error,
};
use ::core::panic;

//==================================================================================================
// Unit Tests
//==================================================================================================

/// Helper test function that creates a [`Bitmap`] from a raw array.
fn test_helper_create_bitmap_from_raw_array(data: &mut [u8]) -> Result<Bitmap, Error> {
    let ptr: *mut u8 = data.as_mut_ptr();
    let len: usize = data.len();
    let array = match unsafe { RawArray::from_raw_parts(ptr, len) } {
        Ok(array) => array,
        Err(e) => return Err(e),
    };

    Ok(Bitmap::from_raw_array(array))
}

/// Attempts to create a [`Bitmap`] from a raw array.
#[test]
fn test_from_raw_array() {
    let mut data: [u8; 4] = [1; 4];

    // Create bitmap.
    let bitmap: Bitmap = match test_helper_create_bitmap_from_raw_array(&mut data) {
        Ok(bitmap) => bitmap,
        Err(_) => panic!("failed to create bitmap"),
    };

    // Check if the bitmap has the expected length.
    if bitmap.number_of_bits() != data.len() * u8::BITS as usize {
        panic!("unexpected length (expected={}, got={})", data.len(), bitmap.number_of_bits());
    }

    // Check if the bitmap was initialized if all bits set to zero.
    for byte in bitmap.iter() {
        if *byte != 0 {
            panic!("unexpected byte value (expected=0, got={})", *byte);
        }
    }
}

/// Attempts to set and clear all bits in a [`Bitmap`].
#[test]
fn test_set_and_clear_all_bits() {
    let mut data: [u8; 4] = [0; 4];

    // Create bitmap.
    let mut bitmap: Bitmap = match test_helper_create_bitmap_from_raw_array(&mut data) {
        Ok(bitmap) => bitmap,
        Err(_) => {
            panic!("failed to create bitmap");
        },
    };

    // Set all bits.
    for i in 0..bitmap.number_of_bits() {
        if bitmap.set(i).is_err() {
            panic!("failed to set bit at index {}", i);
        }
    }

    // Check if all bits were set.
    for byte in bitmap.iter() {
        if *byte != u8::MAX {
            panic!("unexpected byte value (expected={}, got={})", u8::MAX, *byte);
        }
    }

    // Clear all bits.
    for i in 0..bitmap.number_of_bits() {
        if bitmap.clear(i).is_err() {
            panic!("failed to clear bit at index {}", i);
        }
    }

    // Check if all bits were cleared.
    for byte in bitmap.iter() {
        if *byte != 0 {
            panic!("unexpected byte value (expected=0, got={})", *byte);
        }
    }
}

/// Attempts to allocate an clear all bits in a [`Bitmap`].
#[test]
fn test_alloc_and_clear_all_bits() {
    let mut data: [u8; 4] = [0; 4];

    // Create bitmap.
    let mut bitmap: Bitmap = match test_helper_create_bitmap_from_raw_array(&mut data) {
        Ok(bitmap) => bitmap,
        Err(_) => {
            panic!("failed to create bitmap");
        },
    };

    // Allocate all bits.
    for i in 0..bitmap.number_of_bits() {
        if bitmap.alloc().is_err() {
            panic!("failed to allocate bit at index {}", i);
        }
    }

    // Check if all bits were allocated.
    for byte in bitmap.iter() {
        if *byte != u8::MAX {
            panic!("unexpected byte value (expected={}, got={})", u8::MAX, *byte);
        }
    }

    // Clear all bits.
    for i in 0..bitmap.number_of_bits() {
        if bitmap.clear(i).is_err() {
            panic!("failed to clear bit at index {}", i);
        }
    }

    // Check if all bits were cleared.
    for byte in bitmap.iter() {
        if *byte != 0 {
            panic!("unexpected byte value (expected=0, got={})", *byte);
        }
    }
}
