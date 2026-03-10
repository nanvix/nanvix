// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(all(test, feature = "std"), feature(random))]
// To support attributes on statements, e.g., #[verus_spec(invariant ...)] while ...,
// we need `proc_macro_hygiene` and `stmt_expr_attributes`.
#![cfg_attr(verus_keep_ghost, feature(proc_macro_hygiene))]
#![cfg_attr(verus_keep_ghost, feature(stmt_expr_attributes))]
// Verus does not yet support compound assignment on struct fields (e.g., self.usage += 1).
#![allow(clippy::assign_op_pattern)]

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;

//==================================================================================================
// Imports
//==================================================================================================

use ::raw_array::RawArray;
#[cfg(verus_keep_ghost)]
use ::raw_array::{
    axiom_u8_zero_is_0,
    is_zero,
};
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

// Include verified tests.
#[cfg(verus_keep_ghost)]
include!("lib.test.rs");

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A bitmap.
///
#[verus_verify]
//#[verus_verify(ext_equal)] // TODO - Restore when supported by Verus PR #2239
#[verus_verify(external_derive)]
#[derive(Debug)]
pub struct Bitmap {
    /// Capacity of the bitmap (in bits).
    number_of_bits: usize,
    /// Number of bits set in the bitmap.
    usage: usize,
    /// Underlying bits.
    bits: RawArray<u8>,
    /// Hint: first bit index that might be free. Avoids O(n) rescans.
    next_free: usize,
}

//==================================================================================================
// Implementations
//==================================================================================================

#[verus_verify]
impl Bitmap {
    ///
    /// # Description
    ///
    /// Creates a new bitmap with a given length. The bitmap is initialized with all bits set to zero.
    ///
    /// # Parameters
    ///
    /// - `number_of_bits`: Length of the bitmap in bits.
    ///
    /// # Returns
    ///
    /// Upon success, a new bitmap is returned. Upon failure, an error is returned instead.
    ///
    #[verus_spec(result =>
        ensures
            result matches Ok(bitmap) ==> {
                &&& bitmap.inv()
                &&& bitmap@.num_bits == number_of_bits as int
                &&& bitmap@.is_empty()
            },
            number_of_bits == 0 ==> result is Err,
            number_of_bits >= u32::MAX ==> result is Err,
            number_of_bits % (u8::BITS as usize) != 0 ==> result is Err,
    )]
    pub fn new(number_of_bits: usize) -> Result<Self, Error> {
        // Check if the length is invalid.
        if number_of_bits == 0 || number_of_bits >= u32::MAX as usize {
            let reason: &str = "invalid length";
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // Check if the length is not a multiple of the number of the bitmap word.
        if !number_of_bits.is_multiple_of(u8::BITS as usize) {
            let reason: &str = "length must be a multiple of 8";
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // Allocate the bitmap.
        // Note: RawArray::new() guarantees zero-initialization of the backing storage.
        let len: usize = number_of_bits / u8::BITS as usize;
        proof! {
            Self::lemma_u8_array_len_fits_isize(len);
        }
        let array: RawArray<u8> = RawArray::new(len)?;

        let result = Self {
            number_of_bits,
            bits: array,
            usage: 0,
            next_free: 0,
        };

        proof! {
            Self::lemma_new_bitmap_inv(&result);
        }

        Ok(result)
    }

    ///
    /// # Description
    ///
    /// Creates a new bitmap from a raw array. The bitmap is initialized with
    /// all bits set to zero.
    ///
    /// # Parameters
    ///
    /// - `array`: Raw array to create the bitmap from.
    ///
    /// # Returns
    ///
    /// Upon success, a new bitmap is returned. Upon failure, an error is returned instead.
    ///
    /// # Errors
    ///
    /// - `InvalidArgument` if the array length multiplied by 8 overflows `usize`.
    ///
    #[verus_spec(result =>
        requires
            array.inv(),
            array@.len() > 0,
            array@.len() * (u8::BITS as usize) < u32::MAX as usize,
            forall|i: int| 0 <= i < array@.len() ==> array@[i] == 0,
        ensures
            // Liveness: given preconditions, always succeeds.
            result matches Ok(bitmap) && {
                &&& bitmap.inv()
                &&& bitmap@.num_bits == array@.len() * (u8::BITS as int)
                &&& bitmap@.is_empty()
                &&& forall|i: int| 0 <= i < bitmap@.num_bits ==> !bitmap@.is_bit_set(i)
            },
    )]
    pub fn from_raw_array(array: RawArray<u8>) -> Result<Self, Error> {
        // TODO: remove this runtime check once all callers are verified.
        let number_of_bits: usize = match array.len().checked_mul(u8::BITS as usize) {
            Some(n) => n,
            None => {
                let reason: &str = "bitmap size overflow: array too large";
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            },
        };

        // Note: RawArray guarantees zero-initialization of the backing storage.

        let result = Self {
            number_of_bits,
            bits: array,
            usage: 0,
            next_free: 0,
        };

        proof! {
            result.lemma_zero_bytes_means_empty_set();
        }

        Ok(result)
    }

    ///
    /// # Description
    ///
    /// Returns the number of bits in the bitmap.
    ///
    /// # Returns
    ///
    /// The number of bits in the bitmap.
    ///
    #[verus_spec(result =>
        requires
            self.inv(),
        ensures
            result as int == self@.num_bits,
            result > 0,
            result < u32::MAX as usize,
    )]
    pub fn number_of_bits(&self) -> usize {
        self.number_of_bits
    }

    ///
    /// # Description
    ///
    /// Allocates a bit in the bitmap.
    ///
    /// # Returns
    ///
    /// Upon success, the index of the allocated bit is returned. Upon failure, an error is returned
    /// instead.
    ///
    #[verus_spec(result =>
        requires
            old(self).inv(),
        ensures
            self.inv(),
            match result {
                Ok(index) => {
                    &&& 0 <= index < self@.num_bits
                    &&& self@.num_bits == old(self)@.num_bits
                    &&& !old(self)@.is_bit_set(index as int)
                    &&& self@.is_bit_set(index as int)
                    &&& forall|i: int|
                        0 <= i < self@.num_bits && i != index ==> self@.is_bit_set(i) == old(
                            self,
                        )@.is_bit_set(i)
                    &&& self@.set_bits == old(self)@.set_bits.insert(index as int)
                    &&& self@.usage() == old(self)@.usage() + 1
                },
                Err(_) => {
                    &&& old(self)@.is_full()
                    &&& self@ == old(self)@
                },
            },
    )]
    pub fn alloc(&mut self) -> Result<usize, Error> {
        proof! {
            if old(self)@.has_free_bit() {
                old(self)@.lemma_has_free_bit_implies_exists_free_range_1();
            }
        }
        self.alloc_range(1)
    }

    ///
    /// # Description
    ///
    /// Allocates a range of bits in the bitmap.
    ///
    /// # Parameters
    ///
    /// - `size`: Size of the range to allocate.
    ///
    /// # Returns
    ///
    /// Upon success, the index of the allocated range is returned. Upon failure, an error is returned
    /// instead.
    ///
    #[verus_spec(result =>
        requires
            old(self).inv(),
            size > 0,
            size <= old(self)@.num_bits,
        ensures
            self.inv(),
            match result {
                Ok(start) => {
                    &&& 0 <= start < self@.num_bits
                    &&& 0 < size <= self@.num_bits
                    &&& start + (size as int) <= self@.num_bits
                    &&& self@.num_bits == old(self)@.num_bits
                    &&& self@.all_bits_set_in_range(start as int, start + (size as int))
                    &&& old(self)@.all_bits_unset_in_range(
                        start as int,
                        start + (size as int),
                    )
                    // Frame: only the allocated range changed.
                    &&& forall|i: int|
                        0 <= i < self@.num_bits && (i < start || i >= start + (size as int))
                            ==> self@.is_bit_set(i) == old(self)@.is_bit_set(
                            i,
                        )
                    // Set-based frame.
                    &&& self@.set_bits == old(self)@.set_bits.union(
                        BitmapView::range_set(start as int, start + (size as int)),
                    )
                    &&& self@.usage() == old(self)@.usage() + (size as int)
                },
                Err(_) => {
                    &&& !old(self)@.exists_contiguous_free_range(size as int)
                    &&& self@ == old(self)@
                },
            },
    )]
    pub fn alloc_range(&mut self, size: usize) -> Result<usize, Error> {
        // TODO: remove this runtime check once all callers are verified.
        // Check if the size is valid.
        if size == 0 || size > self.number_of_bits {
            proof! {
                if size > self.number_of_bits {
                    self.lemma_no_free_range_when_size_exceeds(size as int);
                }
            }
            let reason: &str = "invalid size";
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // Check if allocation exceeds the bitmap capacity.
        if self.usage > self.number_of_bits - size {
            proof! {
                self.lemma_no_free_range_when_usage_exceeds(size as int);
            }
            let reason: &str = "allocation exceeds bitmap capacity";
            return Err(Error::new(ErrorCode::OutOfMemory, reason));
        }

        // Note: debug_assert_eq! is not supported by Verus, so we guard it
        // with cfg. The invariant self.inv() already proves this property.
        #[cfg(not(verus_keep_ghost))]
        debug_assert_eq!(
            self.bits.len() * u8::BITS as usize,
            self.number_of_bits,
            "bitmap length must match the number of bits"
        );

        let initial_start: usize = self.next_free;
        let mut start: usize = initial_start;
        let mut wrapped: bool = false;
        let mut done: bool = false;

        // Traverse the bitmap, wrapping around once if needed.
        #[cfg_attr(verus_keep_ghost, verus_spec(
            invariant
                self.alloc_range_first_loop_invariant(old(self), size, start, initial_start, wrapped, done),
            decreases
                if !done { 1int } else { 0int },
                if !wrapped { 1int } else { 0int },
                self.number_of_bits - start,
        ))]
        while !done {
            // Stop condition: exceeded the last valid starting position.
            if start > self.number_of_bits - size {
                // If we haven't wrapped yet and started past 0, retry from beginning.
                if !wrapped && initial_start > 0 {
                    proof! {
                        self.lemma_phase1_complete_no_free_range(initial_start, start, size);
                    }
                    start = 0;
                    wrapped = true;
                } else {
                    proof! {
                        self.lemma_all_positions_no_free_range(initial_start, start, size, wrapped);
                    }
                    done = true;
                }
            }

            // After wrap-around, stop if we've reached the initial position.
            if !done && wrapped && start >= initial_start {
                proof! {
                    self.lemma_all_positions_no_free_range(initial_start, start, size, wrapped);
                }
                done = true;
            }

            if !done {
                // Check for fast-skip path.
                let is_aligned: bool = start.is_multiple_of(u8::BITS as usize);
                if is_aligned {
                    let word: usize = start / u8::BITS as usize;
                    // Fast skip: if the starting word is full, skip to the next word.
                    if self.bits[word] == u8::MAX {
                        proof! {
                            self.lemma_full_byte_no_free_range(start as int, size as int);
                        }
                        start += u8::BITS as usize;
                        continue;
                    }
                }

                // Check if all bits in the range are free.
                proof_decl! {
                    let ghost start_before_inner: usize = start;
                    // Ghost: snapshot the "checked before" region for the inner loop.
                    let ghost checked_before: int = start as int;
                }
                let mut offset: usize = 0;
                let mut free: bool = true;

                #[cfg_attr(verus_keep_ghost, verus_spec(
                    invariant
                        self.alloc_range_second_loop_invariant(old(self), size, start, initial_start, offset,
                                                               wrapped, free, checked_before, start_before_inner),
                    invariant_except_break
                        start == start_before_inner,
                        free,
                    ensures
                        self.alloc_range_second_loop_ensures(size, start, initial_start, wrapped,
                                                             free, start_before_inner),
                    decreases size - offset,
                ))]
                while offset < size {
                    let idx: usize = start + offset;
                    let (w, b): (usize, usize) = self.index_unchecked(idx);
                    if (self.bits[w] & (1 << b)) != 0 {
                        free = false;
                        start += offset + 1;
                        proof! {
                            self.lemma_set_bit_blocks_free_range(start_before_inner, idx, offset, size);
                        }
                        break;
                    }
                    offset += 1;
                }

                if free {
                    proof_decl! {
                        // Found a free range at [start, start + size).
                        self.lemma_free_range_was_unset_in_old(old(self), start, size);
                        // Allocate the range.
                        let ghost pre_alloc_self = *self;
                    }

                    #[cfg_attr(verus_keep_ghost, verus_spec(
                        invariant
                            self.alloc_range_third_loop_invariant(old(self), pre_alloc_self, size, start,
                                                                  alloc_offset),
                        decreases size - alloc_offset,
                    ))]
                    for alloc_offset in 0..size {
                        let idx: usize = start + alloc_offset;
                        let (w, b): (usize, usize) = self.index_unchecked(idx);
                        proof_decl! {
                            let ghost loop_old_self = *self;
                        }

                        // Verus note:
                        // `self.bits[w] |= 1 << b` is not supported for mutable index.
                        self.bits.set(w, self.bits[w] | (1 << b));

                        proof! {
                            loop_old_self.lemma_byte_or_reflects_in_view(self, w as int, b as int);
                            self.lemma_alloc_loop_step_inv(old(self), &loop_old_self, start,
                                                           alloc_offset, idx);
                        }
                    }
                    // Verus note: compound assignment on struct fields not supported.
                    self.usage = self.usage + size;
                    self.next_free = start + size;

                    proof! {
                        self.lemma_alloc_range_establishes_inv(old(self), start, size);
                    }

                    return Ok(start);
                }
                // !free: start was advanced past the blocked position.
            }
        }

        // No free range found anywhere in the bitmap.
        proof! {
            self.lemma_no_range_found_frame(old(self), size as int);
        }
        let reason: &str = "bitmap is full";
        Err(Error::new(ErrorCode::OutOfMemory, reason))
    }

    ///
    /// # Description
    ///
    /// Sets a bit at a given index in the bitmap.
    ///
    /// # Parameters
    ///
    /// - `index`: Index of the bit to set.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
    ///
    #[verus_spec(result =>
        requires
            old(self).inv(),
        ensures
            self.inv(),
            match result {
                Ok(()) => {
                    &&& index < self@.num_bits
                    &&& self@.is_bit_set(index as int)
                    &&& !old(self)@.is_bit_set(index as int)
                    &&& self@.num_bits == old(self)@.num_bits
                    // Frame.
                    &&& forall|i: int|
                        0 <= i < self@.num_bits && i != (index as int) ==> self@.is_bit_set(i)
                            == old(self)@.is_bit_set(
                            i,
                        )
                    // Set-based frame.
                    &&& self@.set_bits == old(self)@.set_bits.insert(index as int)
                    &&& self@.usage() == old(self)@.usage() + 1
                },
                Err(_) => {
                    &&& index >= old(self)@.num_bits || old(self)@.is_bit_set(index as int)
                    &&& *self == *old(self)
                },
            },
    )]
    pub fn set(&mut self, index: usize) -> Result<(), Error> {
        // TODO: remove this runtime check once all callers are verified and we add a
        // precondition to this function requiring the bit to already be cleared.

        // Check if the bit is already set.
        if self.test(index)? {
            let reason: &str = "bit is already set";
            return Err(Error::new(ErrorCode::ResourceBusy, reason));
        }
        let (word, bit): (usize, usize) = self.index(index)?;
        proof_decl! {
            let ghost old_self = *self;
            assert(!old_self@.set_bits.contains(index as int));
        }

        self.bits.set(word, self.bits[word] | (1 << bit));

        proof! {
            old_self.lemma_set_bit_preserves_inv(self, word as int, bit as int, index as int);
        }

        self.usage = self.usage + 1;

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Clears a bit at a given index in the bitmap.
    ///
    /// # Parameters
    ///
    /// - `index`: Index of the bit to clear.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
    ///
    #[verus_spec(result =>
        requires
            old(self).inv(),
        ensures
            self.inv(),
            match result {
                Ok(()) => {
                    &&& index < self@.num_bits
                    &&& !self@.is_bit_set(index as int)
                    &&& self@.num_bits == old(self)@.num_bits
                    &&& forall|i: int|
                        0 <= i < self@.num_bits && i != (index as int) ==> self@.is_bit_set(i)
                            == old(self)@.is_bit_set(i)
                    &&& self@.set_bits == old(self)@.set_bits.remove(index as int)
                    &&& self@.usage() == old(self)@.usage() - 1
                },
                Err(_) => {
                    &&& index >= old(self)@.num_bits || !old(self)@.is_bit_set(index as int)
                    &&& *self == *old(self)
                },
            },
    )]
    pub fn clear(&mut self, index: usize) -> Result<(), Error> {
        // TODO: remove this runtime check once all callers are verified and we add a
        // precondition to this function requiring the bit to already be set.

        // Check if the bit is already cleared.
        if !self.test(index)? {
            let reason: &str = "bit is already cleared";
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }
        let (word, bit): (usize, usize) = self.index(index)?;
        proof_decl! {
            let ghost old_self = *self;
            assert(old_self@.set_bits.contains(index as int));
        }

        self.bits.set(word, self.bits[word] & !(1 << bit));

        proof! {
            old_self.lemma_clear_bit_preserves_inv(self, word as int, bit as int, index as int);
        }

        self.usage = self.usage - 1;
        if index < self.next_free {
            self.next_free = index;
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Tests a bit at a given index in the bitmap.
    ///
    /// # Parameters
    ///
    /// - `index`: Index of the bit to test.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(true)` is returned if the bit is set, `Ok(false)` is returned otherwise.
    /// Upon failure, an error is returned instead.
    ///
    #[verus_spec(result =>
        requires
            self.inv(),
        ensures
            match result {
                Ok(b) => {
                    &&& index < self@.num_bits
                    &&& b == self@.is_bit_set(index as int)
                },
                Err(_) => index >= self@.num_bits,
            },
    )]
    pub fn test(&self, index: usize) -> Result<bool, Error> {
        let (word, bit): (usize, usize) = self.index(index)?;
        Ok((self.bits[word] & (1 << bit)) != 0)
    }

    ///
    /// # Description
    ///
    /// Returns the `(word, bit)` pair of a index.
    ///
    /// # Parameters
    ///
    /// - `index`: Index of the bit.
    ///
    /// # Returns
    ///
    /// Upon success, the `(word, bit)` pair of the index is returned. Upon
    /// failure, an error is returned instead.
    ///
    #[verus_spec(result =>
        requires
            self.inv(),
        ensures
            match result {
                Ok((which_byte, which_bit)) => {
                    &&& index < self.number_of_bits
                    &&& which_byte < self.bits@.len()
                    &&& which_bit < u8::BITS as usize
                    &&& which_byte == index as int / (u8::BITS as int)
                    &&& which_bit == index as int % (u8::BITS as int)
                },
                Err(_) => index >= self.number_of_bits,
            },
    )]
    fn index(&self, index: usize) -> Result<(usize, usize), Error> {
        // Check if the index is out of bounds.
        if index >= self.bits.len() * u8::BITS as usize {
            let reason: &str = "index out of bounds";
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        Ok(self.index_unchecked(index))
    }

    ///
    /// # Description
    ///
    /// Returns the `(word, bit)` pair of a index without checking bounds.
    ///
    /// # Parameters
    ///
    /// - `index`: Index of the bit.
    ///
    /// # Returns
    ///
    /// The `(word, bit)` pair of the index.
    ///
    #[verus_spec(result =>
        requires
            index < self.bits@.len() * u8::BITS as usize,
        ensures
            result.0 < self.bits@.len(),
            result.1 < u8::BITS as usize,
            result.0 as int == index as int / (u8::BITS as int),
            result.1 as int == index as int % (u8::BITS as int),
    )]
    fn index_unchecked(&self, index: usize) -> (usize, usize) {
        let word: usize = index / u8::BITS as usize;
        let bit: usize = index % u8::BITS as usize;
        (word, bit)
    }
}

// Deref implementation for test support (external to verification).
#[cfg(test)]
#[cfg_attr(verus_keep_ghost, verifier::external)]
impl ::core::ops::Deref for Bitmap {
    type Target = RawArray<u8>;

    fn deref(&self) -> &Self::Target {
        &self.bits
    }
}
