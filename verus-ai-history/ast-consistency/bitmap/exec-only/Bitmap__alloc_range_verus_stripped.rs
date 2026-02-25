    pub fn alloc_range(&mut self, size: usize) -> Result<usize, Error>
    {
        

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

        // Note: debug_assert_eq! is not supported by Verus, so we guard it
        // with cfg. The invariant self.inv() already proves this property.
        debug_assert_eq!(
            self.bits.len() * u8::BITS as usize,
            self.number_of_bits,
            "bitmap length must match the number of bits"
        );

        let mut start: usize = 0;

        // Traverse the bitmap until the last possible starting bit.
        while start <= self.number_of_bits - size
        {
            // Check for fast skip/ path.
            let is_aligned: bool = start.is_multiple_of(u8::BITS as usize);
            if is_aligned {
                let word: usize = start / u8::BITS as usize;
                // Fast skip: if the starting word is full, skip to the next word.
                if self.bits[word] == u8::MAX {
                    // Jump to next byte boundary.
                    start += u8::BITS as usize;
                    continue;
                }
            }

            // Check if all bits in the range are free.
            
            let mut offset: usize = 0;
            let mut free: bool = true;

            while offset < size
            {
                let idx: usize = start + offset;
                let (w, b): (usize, usize) = self.index_unchecked(idx);
                if (self.bits[w] & (1 << b)) != 0 {
                    free = false;
                    start += offset + 1;
                    break;
                }
                offset += 1;
            }

            if free {
                // Found a free range at [start, start + size).
                // Allocate the range.
                
                let mut alloc_offset: usize = 0;

                // Verus note: `for offset in 0..size` is not supported;
                // `self.bits[w] |= 1 << b` is not supported for mutable index.
                while alloc_offset < size
                {
                    let idx: usize = start + alloc_offset;
                    let (w, b): (usize, usize) = self.index_unchecked(idx);
                    

                    self.bits.set(w, self.bits[w] | (1 << b));

                    alloc_offset += 1;
                }
                // Verus note: compound assignment on struct fields not supported.
                // Equivalent to source's `self.usage += size`.
                self.usage = self.usage + size;

                return Ok(start);
            }
            // !free: start was advanced past the blocked position.
            }

        // No free range found.
        let reason: &str = "bitmap is full";
        Err(Error::new(ErrorCode::OutOfMemory, reason))
    }
