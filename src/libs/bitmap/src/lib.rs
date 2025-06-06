// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(all(test, feature = "std"), feature(random))]

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;

//==================================================================================================
// Imports
//==================================================================================================

use ::raw_array::RawArray;
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
/// A bitmap.
///
#[derive(Debug)]
pub struct Bitmap {
    /// Capacity of the bitmap (in bits).
    number_of_bits: usize,
    /// Number of bits set in the bitmap.
    usage: usize,
    /// Underlying bits.
    bits: RawArray<u8>,
}

//==================================================================================================
// Implementations
//==================================================================================================

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
    pub fn new(number_of_bits: usize) -> Result<Self, Error> {
        // Check if the length is invalid.
        if number_of_bits == 0 || number_of_bits >= u32::MAX as usize {
            let reason: &str = "invalid length";
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // Check if the length is not a multiple of the number of the bitmap word.
        if number_of_bits % u8::BITS as usize != 0 {
            let reason: &str = "length must be a multiple of 8";
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // Allocate the bitmap.
        let mut array: RawArray<u8> = RawArray::new(number_of_bits / u8::BITS as usize)?;

        // Zero out the bitmap.
        for byte in array.iter_mut() {
            *byte = 0;
        }

        Ok(Self {
            number_of_bits,
            bits: array,
            usage: 0,
        })
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
    pub fn from_raw_array(mut array: RawArray<u8>) -> Self {
        // NOTE: no need to test if the length of the raw array is valid, as it is by construction.

        // Zero out the bitmap.
        for byte in array.iter_mut() {
            *byte = 0;
        }

        Self {
            number_of_bits: array.len() * u8::BITS as usize,
            bits: array,
            usage: 0,
        }
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
    pub fn alloc(&mut self) -> Result<usize, Error> {
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
    pub fn alloc_range(&mut self, size: usize) -> Result<usize, Error> {
        // Check if the size is valid.
        if size == 0 || size > self.number_of_bits {
            let reason: &str = "invalid size";
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // Check if allocation exceeds the bitmap capacity.
        if self.usage > self.number_of_bits - size {
            let reason: &str = "allocation exceeds bitmap capacity";
            return Err(Error::new(ErrorCode::OutOfMemory, reason));
        }

        debug_assert_eq!(
            self.bits.len() * u8::BITS as usize,
            self.number_of_bits,
            "bitmap length must match the number of bits"
        );

        let mut start: usize = 0;

        // Traverse the bitmap until the last possible starting bit.
        while start <= self.number_of_bits - size {
            // Check for fast skip/ path.
            let word: usize = start / u8::BITS as usize;
            let is_aligned: bool = start % u8::BITS as usize == 0;
            if is_aligned as usize == 0 {
                // Fast skip: if the starting word is full, skip to the next word.
                if self.bits[word] == u8::MAX {
                    // Jump to next byte boundary.
                    let next_start: usize = (word + 1) * u8::BITS as usize;
                    start = next_start;
                    continue;
                }
            }

            // Check if all bits in the range are free.
            let mut free: bool = true;
            for offset in 0..size {
                let idx: usize = start + offset;
                let (w, b): (usize, usize) = self.index_unchecked(idx);
                if (self.bits[w] & (1 << b)) != 0 {
                    free = false;
                    start += offset + 1;
                    break;
                }
            }
            if free {
                // Allocate the range
                for offset in 0..size {
                    let idx: usize = start + offset;
                    let (w, b): (usize, usize) = self.index_unchecked(idx);
                    self.bits[w] |= 1 << b;
                }
                self.usage += size;
                return Ok(start);
            }
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
    pub fn set(&mut self, index: usize) -> Result<(), Error> {
        // Check if the bit is already set.
        if self.test(index)? {
            let reason: &str = "bit is already set";
            return Err(Error::new(ErrorCode::ResourceBusy, reason));
        }
        let (word, bit): (usize, usize) = self.index(index)?;
        self.bits[word] |= 1 << bit;
        self.usage += 1;
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
    pub fn clear(&mut self, index: usize) -> Result<(), Error> {
        // Check if the bit is already cleared.
        if !self.test(index)? {
            let reason: &str = "bit is already cleared";
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }
        let (word, bit): (usize, usize) = self.index(index)?;
        self.bits[word] &= !(1 << bit);
        self.usage -= 1;
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
    fn index_unchecked(&self, index: usize) -> (usize, usize) {
        let word: usize = index / u8::BITS as usize;
        let bit: usize = index % u8::BITS as usize;
        (word, bit)
    }
}

#[cfg(test)]
impl ::core::ops::Deref for Bitmap {
    type Target = RawArray<u8>;

    fn deref(&self) -> &Self::Target {
        &self.bits
    }
}
