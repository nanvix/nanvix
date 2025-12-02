// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

//==================================================================================================
// Imports
//==================================================================================================

use crate::Bitmap;
use ::rand::Rng;
use ::raw_array::RawArray;
use ::sys::error::Error;

//==================================================================================================
// Unit Tests
//==================================================================================================

/// Helper test function that creates a [`Bitmap`] from a raw array.
fn test_helper_create_bitmap_from_raw_array(data: &mut [u8]) -> Result<Bitmap, Error> {
    let ptr: *mut u8 = data.as_mut_ptr();
    let len: usize = data.len();
    let array = unsafe { RawArray::from_raw_parts(ptr, len)? };

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

/// Attempts to allocate a range of bits that crosses a word boundary in a [`Bitmap`].
#[test]
fn test_alloc_range_across_word_boundary() {
    let mut data: [u8; 4] = [0; 4];

    // Create bitmap.
    let mut bitmap: Bitmap = match test_helper_create_bitmap_from_raw_array(&mut data) {
        Ok(bitmap) => bitmap,
        Err(_) => panic!("failed to create bitmap"),
    };

    // Choose a range that crosses a byte (word) boundary, e.g., bits 6..10 (crosses from byte 0 to byte 1)
    let start = 6;
    let end = 10;

    // Set all bits that are not in the range.
    for i in 0..bitmap.number_of_bits() {
        if i < start || i >= end {
            if bitmap.set(i).is_err() {
                panic!("failed to set bit at index {i}");
            }
        }
    }

    // Attempt to allocate the range.
    if bitmap.alloc_range(end - start).is_err() {
        panic!("failed to allocate range (start={start}, end={end})");
    }

    // Check if the bits in the range were allocated.
    for i in start..end {
        if !bitmap.test(i).unwrap_or(false) {
            panic!("bit at index {i} should be allocated but is not");
        }
    }
}

/// Attempts to allocate a range larger than a byte (should fail).
#[test]
fn test_alloc_range_too_large() {
    let mut data: [u8; 4] = [0; 4];
    let mut bitmap = test_helper_create_bitmap_from_raw_array(&mut data).unwrap();
    let result = bitmap.alloc_range(data.len() * u8::BITS as usize + 1);
    assert!(result.is_err(), "alloc_range should fail for size > u8::BITS");
}

/// Attempts to allocate a range of size zero (should fail).
#[test]
fn test_alloc_range_zero() {
    let mut data: [u8; 4] = [0; 4];
    let mut bitmap = test_helper_create_bitmap_from_raw_array(&mut data).unwrap();
    let result = bitmap.alloc_range(0);
    assert!(result.is_err(), "alloc_range should fail for size 0");
}

/// Attempts to allocate random ranges in a cleared [`Bitmap`].
#[test]
fn test_alloc_random_ranges() {
    let mut data: [u8; 4] = [0; 4];
    const NUMBER_OF_ITERATIONS: usize = 1000;

    // Create bitmap.
    let mut bitmap: Bitmap = match test_helper_create_bitmap_from_raw_array(&mut data) {
        Ok(bitmap) => bitmap,
        Err(_) => panic!("failed to create bitmap"),
    };

    // Allocate random ranges.
    for _ in 0..NUMBER_OF_ITERATIONS {
        let size: usize = (::rand::thread_rng().gen::<usize>() % bitmap.number_of_bits()) + 1;

        let start: usize = bitmap.alloc_range(size).unwrap_or_else(|_| {
            panic!("failed to allocate range of size {}", size);
        });

        // Check if the bits in the range were allocated.
        for i in start..(start + size) {
            if !bitmap.test(i).unwrap_or(false) {
                panic!("bit at index {} should be allocated but is not", i);
            }
        }

        // Clear the range after testing.
        for i in start..(start + size) {
            if bitmap.clear(i).is_err() {
                panic!("failed to clear bit at index {}", i);
            }
        }
    }
}

/// Attempts to allocate random bits in a partially set [`Bitmap`].
#[test]
fn test_alloc_random_bits_in_partial_bitmap() {
    let mut data: [u8; 4] = [0; 4];
    const NUMBER_OF_ITERATIONS: usize = 1000;

    // Create bitmap.
    let mut bitmap: Bitmap = match test_helper_create_bitmap_from_raw_array(&mut data) {
        Ok(bitmap) => bitmap,
        Err(_) => panic!("failed to create bitmap"),
    };

    // Set some bits randomly.
    for _ in 0..data.len() / 2 {
        let index: usize = ::rand::thread_rng().gen::<usize>() % bitmap.number_of_bits();
        let _ = bitmap.set(index);
    }

    // Allocate random bits.
    for _ in 0..NUMBER_OF_ITERATIONS {
        let index: usize = bitmap.alloc().unwrap_or_else(|_| {
            panic!("failed to allocate bit");
        });

        // Check if the bit was allocated.
        if !bitmap.test(index).unwrap_or(false) {
            panic!("bit at index {} should be allocated but is not", index);
        }

        // Clear the bit after testing.
        if bitmap.clear(index).is_err() {
            panic!("failed to clear bit at index {}", index);
        }
    }
}

/// Attempts to allocate random ranges in a partially set [`Bitmap`].
#[test]
fn test_alloc_random_ranges_in_partial_bitmap() {
    let mut data: [u8; 4] = [0; 4];
    const NUMBER_OF_ITERATIONS: usize = 1000;

    // Create bitmap.
    let mut bitmap: Bitmap = match test_helper_create_bitmap_from_raw_array(&mut data) {
        Ok(bitmap) => bitmap,
        Err(_) => panic!("failed to create bitmap"),
    };

    for _ in 0..NUMBER_OF_ITERATIONS {
        // Choose a range that crosses a byte (word) boundary, e.g., bits 6..10 (crosses from byte 0 to byte 1)
        let start = ::rand::thread_rng().gen::<usize>() % bitmap.number_of_bits();
        let end =
            start + (::rand::thread_rng().gen::<usize>() % (bitmap.number_of_bits() - start)) + 1;

        // Set all bits that are not in the range.
        for i in 0..bitmap.number_of_bits() {
            if i < start || i >= end {
                if bitmap.set(i).is_err() {
                    panic!("failed to set bit at index {i} {start} {end}");
                }
            }
        }

        // Attempt to allocate the range.
        if bitmap.alloc_range(end - start).is_err() {
            panic!("failed to allocate range (start={start}, end={end})");
        }

        // Check if the bits in the range were allocated.
        for i in start..end {
            if !bitmap.test(i).unwrap_or(false) {
                panic!("bit at index {i} should be allocated but is not");
            }
        }

        // Clear the range after testing.
        for i in 0..bitmap.number_of_bits() {
            if bitmap.clear(i).is_err() {
                panic!("failed to clear bit at index {}", i);
            }
        }
    }
}
