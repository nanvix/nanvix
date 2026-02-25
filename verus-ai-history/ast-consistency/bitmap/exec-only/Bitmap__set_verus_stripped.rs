    pub fn set(&mut self, index: usize) -> Result<(), Error>
    {
        // Check if the bit is already set.
        if self.test(index)? {
            let reason: &str = "bit is already set";
            return Err(Error::new(ErrorCode::ResourceBusy, reason));
        }

        let (word, bit): (usize, usize) = self.index(index)?;
        

        // At this point, we know:
        // - old_self.inv() holds
        // - !old_self.is_bit_set(index as int) (the bit is not set)
        self.bits.set(word, self.bits[word] | (1 << bit));

        self.usage = self.usage + 1;

        Ok(())
    }
