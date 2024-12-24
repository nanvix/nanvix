// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(test)]
mod test;

//==================================================================================================
// Imports
//==================================================================================================

use crate::collections::raw_array::RawArray;
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
    /// - `len`: Length of the bitmap in bits.
    ///
    /// # Returns
    ///
    /// Upon success, a new bitmap is returned. Upon failure, an error is returned instead.
    ///
    pub fn new(len: usize) -> Result<Self, Error> {
        // Check if the length is invalid.
        if len == 0 || len >= i32::MAX as usize {
            let reason: &str = "invalid length";
            trace!("new(): {reason} (len={len})");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // Allocate the bitmap.
        let mut array: RawArray<u8> = RawArray::new(len)?;

        // Zero out the bitmap.
        for byte in array.iter_mut() {
            *byte = 0;
        }

        Ok(Self {
            number_of_bits: array.len() * u8::BITS as usize,
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
        // TODO: optimize this implementation to traverse the bitmap in a machine word at a time.

        // Traverse the bitmap one word at a time.
        for (i, word) in self.bits.iter_mut().enumerate() {
            // Check if this word is not full.
            if *word != u8::MAX {
                // Find a free bit.
                for j in 0..u8::BITS as usize {
                    // Check if the bit is free.
                    if *word & (1 << j) == 0 {
                        // It is, thus allocate it.
                        *word |= 1 << j;
                        self.usage += 1;
                        return Ok(i * u8::BITS as usize + j);
                    }
                }
            }
        }

        let reason: &str = "bitmap is full";
        trace!("alloc(): {reason}");
        Err(Error::new(ErrorCode::OutOfMemory, reason))
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
        // Check if the size is invalid.
        if (size == 0) || (size > u8::BITS as usize) {
            let reason: &str = "invalid size";
            trace!("alloc_range(): {reason} (size={size})");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // Check if the size is out of bounds.
        if size > self.number_of_bits {
            let reason: &str = "size out of bounds";
            trace!("alloc_range(): {reason} (size={size})");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // Compute mask.
        let mask: u8 = if size == u8::BITS as usize {
            // Use u8::MAX directly to avoid overflow
            u8::MAX
        } else {
            ((1 << size) - 1) as u8
        };

        // Traverse the bitmap one word at a time.
        for (i, word) in self.bits.iter_mut().enumerate() {
            // Check if this word is not full.
            if *word != u8::MAX {
                // Find a free range of bits.
                for j in 0..=(u8::BITS as usize - size) {
                    // Check if the range is free.
                    if (*word & (mask << j)) == 0 {
                        // It is, thus allocate it.
                        *word |= mask << j;
                        self.usage += size;
                        return Ok(i * u8::BITS as usize + j);
                    }
                }
            }
        }

        let reason: &str = "bitmap is full";
        trace!("alloc_range(): {reason}");
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
            trace!("set(): {reason} (index={index})");
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
            trace!("clear(): {reason} (index={index})");
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
            trace!("index(): {reason} (index={index})");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        let word: usize = index / u8::BITS as usize;
        let bit: usize = index % u8::BITS as usize;

        Ok((word, bit))
    }
}

#[cfg(test)]
impl ::core::ops::Deref for Bitmap {
    type Target = RawArray<u8>;

    fn deref(&self) -> &Self::Target {
        &self.bits
    }
}
