// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// Bitmap Allocator - Verified Tests.
// Verified test functions that prove key bitmap properties.

verus! {

//==================================================================================================
// Verified Test Functions
//==================================================================================================

/// Verifiable test: setting and clearing a bit should work correctly.
fn test_bitmap_set_clear_verified(number_of_bits: usize, index: usize)
    requires
        number_of_bits > 0,
        number_of_bits < u32::MAX as usize,
        number_of_bits % (u8::BITS as usize) == 0,
        index < number_of_bits,
{
    let result: Result<Bitmap, Error> = Bitmap::new(number_of_bits);
    if let Ok(mut bitmap) = result {
        // Set the bit.
        let set_result: Result<(), Error> = bitmap.set(index);
        if let Ok(()) = set_result {
            // The bit should be set.
            assert(bitmap@.is_bit_set(index as int));

            // Clear the bit.
            let clear_result: Result<(), Error> = bitmap.clear(index);
            if let Ok(()) = clear_result {
                // The bit should be cleared.
                assert(!bitmap@.is_bit_set(index as int));
            }
        }
    }
}

/// Verifiable test: allocating a range should allocate contiguous bits.
fn test_bitmap_alloc_range_verified(number_of_bits: usize, size: usize)
    requires
        number_of_bits > 0,
        number_of_bits < u32::MAX as usize,
        number_of_bits % (u8::BITS as usize) == 0,
        size > 0,
        size <= number_of_bits,
{
    let result: Result<Bitmap, Error> = Bitmap::new(number_of_bits);
    if let Ok(mut bitmap) = result {
        let alloc_result: Result<usize, Error> = bitmap.alloc_range(size);
        if let Ok(start_index) = alloc_result {
            // The start index should be within valid range.
            assert(start_index + size <= number_of_bits);

            // All bits in the range should be set.
            assert(bitmap@.all_bits_set_in_range(start_index as int, (start_index + size) as int));
        }
    }
}

/// Verifiable test: multiple allocations should not overlap.
fn test_bitmap_multiple_alloc_verified(number_of_bits: usize)
    requires
        number_of_bits >= 8,  // Need at least 8 bits (minimum valid bitmap size).
        number_of_bits < u32::MAX as usize,
        number_of_bits % (u8::BITS as usize) == 0,
{
    let result: Result<Bitmap, Error> = Bitmap::new(number_of_bits);
    if let Ok(mut bitmap) = result {
        let alloc1: Result<usize, Error> = bitmap.alloc();
        if let Ok(index1) = alloc1 {
            let alloc2: Result<usize, Error> = bitmap.alloc();
            if let Ok(index2) = alloc2 {
                // The two allocated indices should be different.
                assert(index1 != index2);
                // Both bits should be set.
                assert(bitmap@.is_bit_set(index1 as int));
                assert(bitmap@.is_bit_set(index2 as int));
            }
        }
    }
}

/// Verifiable test: clearing and re-allocating should work.
fn test_bitmap_clear_and_realloc_verified(number_of_bits: usize, index: usize)
    requires
        number_of_bits > 0,
        number_of_bits < u32::MAX as usize,
        number_of_bits % (u8::BITS as usize) == 0,
        index < number_of_bits,
{
    let result: Result<Bitmap, Error> = Bitmap::new(number_of_bits);
    if let Ok(mut bitmap) = result {
        // Set a bit.
        let set_result: Result<(), Error> = bitmap.set(index);
        if let Ok(()) = set_result {
            // Clear the bit.
            let clear_result: Result<(), Error> = bitmap.clear(index);
            if let Ok(()) = clear_result {
                // The bit should be cleared.
                assert(!bitmap@.is_bit_set(index as int));

                // Usage should be back to 0.
                assert(bitmap@.usage() == 0);
            }
        }
    }
}

/// Verifiable test: setting all bits and then clearing all bits.
fn test_set_and_clear_all_bits_verified(number_of_bits: usize)
    requires
        number_of_bits > 0,
        number_of_bits < u32::MAX as usize,
        number_of_bits % (u8::BITS as usize) == 0,
{
    let result: Result<Bitmap, Error> = Bitmap::new(number_of_bits);
    if let Ok(mut bitmap) = result {
        // Set all bits.
        let mut i: usize = 0;
        while i < number_of_bits
            invariant
                0 <= i <= number_of_bits,
                bitmap.inv(),
                bitmap@.num_bits == number_of_bits as int,
                forall|j: int| 0 <= j < i as int ==> bitmap@.is_bit_set(j),
            decreases number_of_bits - i,
        {
            let set_result: Result<(), Error> = bitmap.set(i);
            if let Ok(()) = set_result {
                i = i + 1;
            } else {
                break;
            }
        }

        // If we set all bits successfully.
        if i == number_of_bits {
            // All bits should be set.
            assert(bitmap@.all_bits_set_in_range(0, number_of_bits as int));

            // Clear all bits.
            let mut j: usize = 0;
            while j < number_of_bits
                invariant
                    0 <= j <= number_of_bits,
                    bitmap.inv(),
                    bitmap@.num_bits == number_of_bits as int,
                    forall|k: int| 0 <= k < j as int ==> !bitmap@.is_bit_set(k),
                    forall|k: int| j as int <= k < number_of_bits as int ==> bitmap@.is_bit_set(k),
                decreases number_of_bits - j,
            {
                let clear_result: Result<(), Error> = bitmap.clear(j);
                if let Ok(()) = clear_result {
                    j = j + 1;
                } else {
                    break;
                }
            }

            // If we cleared all bits successfully.
            if j == number_of_bits {
                // All bits should be cleared.
                assert(bitmap@.all_bits_unset_in_range(0, number_of_bits as int));
            }
        }
    }
}

/// Verifiable test: allocating all bits and then clearing all bits.
fn test_alloc_and_clear_all_bits_verified(number_of_bits: usize)
    requires
        number_of_bits > 0,
        number_of_bits < u32::MAX as usize,
        number_of_bits % (u8::BITS as usize) == 0,
{
    let result: Result<Bitmap, Error> = Bitmap::new(number_of_bits);
    if let Ok(mut bitmap) = result {
        // Allocate all bits.
        let mut count: usize = 0;
        while count < number_of_bits
            invariant
                0 <= count <= number_of_bits,
                bitmap.inv(),
                bitmap@.num_bits == number_of_bits as int,
                bitmap@.usage() == count as int,
            decreases number_of_bits - count,
        {
            let alloc_result: Result<usize, Error> = bitmap.alloc();
            if let Ok(_) = alloc_result {
                count = count + 1;
            } else {
                break;
            }
        }

        // If we allocated all bits successfully.
        if count == number_of_bits {
            // Usage should equal number_of_bits.
            assert(bitmap@.usage() == number_of_bits as int);

            // All bits must be set (usage == number_of_bits implies full).
            proof {
                bitmap@.lemma_usage_equals_number_of_bits_implies_full();
            }

            // Clear all bits.
            let mut j: usize = 0;
            while j < number_of_bits
                invariant
                    0 <= j <= number_of_bits,
                    bitmap.inv(),
                    bitmap@.num_bits == number_of_bits as int,
                    forall|k: int| 0 <= k < j as int ==> !bitmap@.is_bit_set(k),
                    forall|k: int| j as int <= k < number_of_bits as int ==> bitmap@.is_bit_set(k),
                decreases number_of_bits - j,
            {
                let clear_result: Result<(), Error> = bitmap.clear(j);
                if let Ok(()) = clear_result {
                    j = j + 1;
                } else {
                    break;
                }
            }

            // If we cleared all bits successfully.
            if j == number_of_bits {
                // All bits should be cleared.
                assert(bitmap@.all_bits_unset_in_range(0, number_of_bits as int));
            }
        }
    }
}

/// Verifiable test: allocating a range that crosses a word boundary.
fn test_alloc_range_across_word_boundary_verified(number_of_bits: usize, start: usize, end: usize)
    requires
        number_of_bits >= 8,
        number_of_bits < u32::MAX as usize,
        number_of_bits % (u8::BITS as usize) == 0,
        start < end,
        end <= number_of_bits,
        // Ensure the range crosses a byte boundary (e.g., bits 6..10).
        start / 8 < (end - 1) / 8,
{
    let result: Result<Bitmap, Error> = Bitmap::new(number_of_bits);
    if let Ok(mut bitmap) = result {
        // Set all bits that are not in the range [start, end).
        let mut i: usize = 0;
        while i < number_of_bits
            invariant
                0 <= i <= number_of_bits,
                bitmap.inv(),
                bitmap@.num_bits == number_of_bits as int,
            decreases number_of_bits - i,
        {
            if i < start || i >= end {
                let _ = bitmap.set(i);
            }
            i = i + 1;
        }

        // Attempt to allocate the range.
        let size: usize = end - start;
        let alloc_result: Result<usize, Error> = bitmap.alloc_range(size);
        if let Ok(alloc_start) = alloc_result {
            // The allocated range should have all bits set.
            assert(bitmap@.all_bits_set_in_range(alloc_start as int, (alloc_start + size) as int));
        }
    }
}

/// Verifiable test: allocating a bit in a partially filled bitmap.
fn test_alloc_in_partial_bitmap_verified(number_of_bits: usize, set_index: usize)
    requires
        number_of_bits >= 8,
        number_of_bits < u32::MAX as usize,
        number_of_bits % (u8::BITS as usize) == 0,
        set_index < number_of_bits,
{
    let result: Result<Bitmap, Error> = Bitmap::new(number_of_bits);
    if let Ok(mut bitmap) = result {
        // Set one bit to partially fill the bitmap.
        let set_result: Result<(), Error> = bitmap.set(set_index);
        if let Ok(()) = set_result {
            // The set bit should be marked as set.
            assert(bitmap@.is_bit_set(set_index as int));

            // Allocate a new bit.
            let alloc_result: Result<usize, Error> = bitmap.alloc();
            if let Ok(index) = alloc_result {
                // The allocated bit should be set.
                assert(bitmap@.is_bit_set(index as int));

                // Clear the allocated bit.
                let clear_result: Result<(), Error> = bitmap.clear(index);
                if let Ok(()) = clear_result {
                    assert(!bitmap@.is_bit_set(index as int));
                    // The originally set bit should still be set (if it wasn't the one we allocated).
                    if index != set_index {
                        assert(bitmap@.is_bit_set(set_index as int));
                    }
                }
            }
        }
    }
}

/// Verifiable test: allocating a range and then clearing it.
fn test_alloc_range_and_clear_verified(number_of_bits: usize, size: usize)
    requires
        number_of_bits > 0,
        number_of_bits < u32::MAX as usize,
        number_of_bits % (u8::BITS as usize) == 0,
        size > 0,
        size <= number_of_bits,
{
    let result: Result<Bitmap, Error> = Bitmap::new(number_of_bits);
    if let Ok(mut bitmap) = result {
        let alloc_result: Result<usize, Error> = bitmap.alloc_range(size);
        if let Ok(start) = alloc_result {
            // Verify the range is allocated.
            assert(bitmap@.all_bits_set_in_range(start as int, (start + size) as int));

            // Clear the range.
            let mut i: usize = start;
            let end: usize = start + size;
            while i < end
                invariant
                    start <= i <= end,
                    end == start + size,
                    end <= number_of_bits,
                    bitmap.inv(),
                    bitmap@.num_bits == number_of_bits as int,
                    forall|j: int| start as int <= j < i as int ==> !bitmap@.is_bit_set(j),
                    forall|j: int| i as int <= j < end as int ==> bitmap@.is_bit_set(j),
                decreases end - i,
            {
                let clear_result: Result<(), Error> = bitmap.clear(i);
                if let Ok(()) = clear_result {
                    i = i + 1;
                } else {
                    break;
                }
            }

            // If we cleared all bits successfully.
            if i == end {
                // All bits in the range should be cleared.
                assert(bitmap@.all_bits_unset_in_range(start as int, end as int));
            }
        }
    }
}

/// Verifiable test: usage tracking is correct.
fn test_bitmap_usage_tracking_verified(number_of_bits: usize)
    requires
        number_of_bits >= 8,  // Need at least 8 bits (minimum valid bitmap size).
        number_of_bits < u32::MAX as usize,
        number_of_bits % (u8::BITS as usize) == 0,
{
    let result: Result<Bitmap, Error> = Bitmap::new(number_of_bits);
    if let Ok(mut bitmap) = result {
        // Initially empty.
        assert(bitmap@.is_empty());
        assert(bitmap@.usage() == 0);

        // Allocate first bit.
        let alloc1: Result<usize, Error> = bitmap.alloc();
        if let Ok(_) = alloc1 {
            assert(bitmap@.usage() == 1);

            // Allocate second bit.
            let alloc2: Result<usize, Error> = bitmap.alloc();
            if let Ok(_) = alloc2 {
                assert(bitmap@.usage() == 2);

                // Allocate third bit.
                let alloc3: Result<usize, Error> = bitmap.alloc();
                if let Ok(index3) = alloc3 {
                    assert(bitmap@.usage() == 3);

                    // Clear one bit.
                    let clear_result: Result<(), Error> = bitmap.clear(index3);
                    if let Ok(()) = clear_result {
                        assert(bitmap@.usage() == 2);
                    }
                }
            }
        }
    }
}

/// Verifiable test: alloc_range preserves bits outside the allocated range.
fn test_bitmap_alloc_range_preserves_others_verified(
    number_of_bits: usize,
    size: usize,
    test_index: usize,
)
    requires
        number_of_bits >= 8,
        number_of_bits < u32::MAX as usize,
        number_of_bits % (u8::BITS as usize) == 0,
        size > 0,
        size < number_of_bits,
        test_index < number_of_bits,
{
    let result: Result<Bitmap, Error> = Bitmap::new(number_of_bits);
    if let Ok(mut bitmap) = result {
        // Set a bit first.
        let set_result: Result<(), Error> = bitmap.set(test_index);
        if let Ok(()) = set_result {
            // Allocate a range.
            let alloc_result: Result<usize, Error> = bitmap.alloc_range(size);
            if let Ok(start_index) = alloc_result {
                // If test_index is outside the allocated range, it should still be set.
                if test_index < start_index || test_index >= start_index + size {
                    assert(bitmap@.is_bit_set(test_index as int));
                }
            }
        }
    }
}

/// Verifiable test: number_of_bits remains constant across operations.
fn test_bitmap_number_of_bits_constant_verified(number_of_bits: usize, index: usize)
    requires
        number_of_bits > 0,
        number_of_bits < u32::MAX as usize,
        number_of_bits % (u8::BITS as usize) == 0,
        index < number_of_bits,
{
    let result: Result<Bitmap, Error> = Bitmap::new(number_of_bits);
    if let Ok(mut bitmap) = result {
        let ghost initial_bits: int = bitmap@.num_bits;
        assert(initial_bits == number_of_bits as int);

        // After allocation.
        let alloc_result: Result<usize, Error> = bitmap.alloc();
        if let Ok(_) = alloc_result {
            assert(bitmap@.num_bits == initial_bits);

            // After setting a bit.
            let set_result: Result<(), Error> = bitmap.set(index);
            match set_result {
                Ok(()) => {
                    assert(bitmap@.num_bits == initial_bits);
                },
                Err(_) => {
                    // If set failed (bit already set), number_of_bits should still be the same.
                    assert(bitmap@.num_bits == initial_bits);
                },
            }
        }
    }
}

} // verus!
